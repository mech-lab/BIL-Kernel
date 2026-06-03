"""AXLE client for the Lean verification API."""

import asyncio
import json
import logging
import os
import warnings
from http import HTTPMethod
from typing import Any, Final, cast

import aiohttp
import httpx
import requests
from tenacity import (
    before_sleep_log,
    retry,
    retry_if_exception_type,
    stop_after_delay,
    wait_exponential_jitter,
)

from axle.exceptions import (
    AxleConflictError,
    AxleForbiddenError,
    AxleInternalError,
    AxleInvalidArgument,
    AxleIsUnavailable,
    AxleNotFoundError,
    AxleRateLimitedError,
    AxleRuntimeError,
)
from axle.types import (
    CheckResponse,
    DisproveResponse,
    ExtractDeclsResponse,
    ExtractTheoremsResponse,
    Have2LemmaResponse,
    Have2SorryResponse,
    MergeResponse,
    NormalizeResponse,
    RenameResponse,
    RepairProofsResponse,
    SimplifyTheoremsResponse,
    Sorry2LemmaResponse,
    Theorem2LemmaResponse,
    Theorem2SorryResponse,
    VerifyProofResponse,
)

JsonDict = dict[str, Any]

logger = logging.getLogger(__name__)


class AxleClient:
    """Client for the AXLE HTTP API."""

    DEFAULT_URL: Final[str] = "https://axle.axiommath.ai"
    # The base value to use when computing the client-side timeout. On the
    # server side, requests are queued and processed in-order. This timeout is
    # intended to be generous enough to allow for queueing delay, retry logic,
    # network overhead, etc.
    BASE_TIMEOUT_SECONDS: Final[float] = 1_800
    # Maximum number of concurrent requests to the server.
    MAX_CONCURRENCY: Final[int] = 20

    def __init__(
        self,
        url: str | None = None,
        max_concurrency: int | None = None,
        base_timeout_seconds: float | None = None,
        api_key: str | None = None,
        http2: bool = True,
    ) -> None:
        """Constructor.

        Please call close() when you are done.

        Args:
            url: The URL of the AXLE server. Defaults to AXLE_API_URL env var
                or https://axle.axiommath.ai.
            max_concurrency: The maximum number of concurrent in-flight requests
                this client will send at once. Enforced with an internal
                semaphore, so the cap is uniform across HTTP/1.1 and HTTP/2.
                Defaults to AXLE_MAX_CONCURRENCY env var or 20.
            base_timeout_seconds: The base timeout in seconds for requests.
                Defaults to AXLE_TIMEOUT_SECONDS env var or 1_800.
            api_key: The API key to use for authentication.
                Defaults to AXLE_API_KEY env var.
                If not provided, no authentication will be used.
            http2: Use HTTP/2 multiplexing via httpx. On by default.
        """
        if url is None:
            url = os.environ.get("AXLE_API_URL", self.DEFAULT_URL)

        while url.endswith("/"):
            url = url[:-1]

        self.url = url
        self._http2 = http2
        # Defer creation of the session because it requires an async event loop,
        # which may not be available at construction time. Holds either an
        # aiohttp.ClientSession or httpx.AsyncClient.
        self._session: aiohttp.ClientSession | httpx.AsyncClient | None = None

        if max_concurrency is None:
            max_concurrency = int(os.environ.get("AXLE_MAX_CONCURRENCY", self.MAX_CONCURRENCY))
        self.max_concurrency = max_concurrency

        # Gates concurrent in-flight requests.
        self._sem = asyncio.Semaphore(max_concurrency)
        self._session_lock = asyncio.Lock()

        if base_timeout_seconds is None:
            base_timeout_seconds = float(
                os.environ.get("AXLE_TIMEOUT_SECONDS", self.BASE_TIMEOUT_SECONDS)
            )
        self.base_timeout_seconds = base_timeout_seconds

        if api_key is None:
            api_key = os.environ.get("AXLE_API_KEY", None)
        self._headers: dict[str, str] = {
            "X-Request-Source": os.environ.get("AXLE_REQUEST_SOURCE", "sdk"),
        }
        if api_key:
            self._headers["Authorization"] = f"Bearer {api_key}"

    def check_status(self, timeout_seconds: float = 60) -> JsonDict:
        """Health check, raising AxleInternalError on error."""
        try:
            response = requests.get(
                f"{self.url}/v1/status", timeout=timeout_seconds, headers=self._headers
            )
        except requests.ConnectionError as e:
            raise AxleIsUnavailable(self.url, str(e)) from e
        if response.status_code != 200:
            if response.status_code == 503:
                raise AxleIsUnavailable(self.url, str(response.text))
            raise AxleInternalError(f"Server error {response.status_code}: {response.text}")
        status: JsonDict = json.loads(response.text)
        if status.get("status") != "healthy":
            raise AxleInternalError(f"Server is not healthy: {status}")
        return status

    async def run_one(self, method: str, request: JsonDict) -> JsonDict:
        """Run a single API request."""
        # Extract timeout from request for client-side timeout calculation
        request_timeout_seconds = request.get("timeout_seconds")

        response_text = await self._call(
            f"api/v1/{method}", request_timeout_seconds, data=json.dumps(request)
        )

        # Parse response and validate single response The server should only
        # return one line of JSON for a single request
        stripped_response = response_text.rstrip()
        lines = stripped_response.split("\n") if stripped_response else []

        if len(lines) != 1:
            raise AxleInternalError(f"Expected 1 response, got {len(lines)}")

        response: JsonDict = json.loads(lines[0])

        if "internal_error" in response:
            raise AxleInternalError(response["internal_error"])

        if "user_error" in response:
            raise AxleInvalidArgument(response["user_error"])

        if "error" in response:
            raise AxleRuntimeError(response["error"])

        return response

    # Public API convenience methods:

    async def verify_proof(
        self,
        formal_statement: str,
        content: str,
        environment: str,
        permitted_sorries: list[str] | None = None,
        mathlib_options: bool | None = None,
        use_def_eq: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> VerifyProofResponse:
        """Verify a proof against a formal statement."""
        return VerifyProofResponse.from_response(
            await self.run_one(
                "verify_proof",
                _to_request(
                    content=content,
                    formal_statement=formal_statement,
                    permitted_sorries=permitted_sorries,
                    mathlib_options=mathlib_options,
                    use_def_eq=use_def_eq,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def extract_theorems(
        self,
        content: str,
        environment: str,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> ExtractTheoremsResponse:
        """Extract theorems with dependencies from Lean code.

        Deprecated: use extract_decls instead, which supports all declaration kinds.
        """
        warnings.warn(
            "extract_theorems is deprecated and will be removed in a future release. "
            "Use extract_decls instead.",
            DeprecationWarning,
            stacklevel=2,
        )
        return ExtractTheoremsResponse.from_response(
            await self.run_one(
                "extract_theorems",
                _to_request(
                    content=content,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def extract_decls(
        self,
        content: str,
        environment: str,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> ExtractDeclsResponse:
        """Extract all declarations with dependencies from Lean code."""
        return ExtractDeclsResponse.from_response(
            await self.run_one(
                "extract_decls",
                _to_request(
                    content=content,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def merge(
        self,
        documents: list[str],
        environment: str,
        use_def_eq: bool | None = None,
        include_alts_as_comments: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> MergeResponse:
        """Merge multiple Lean files into one."""
        return MergeResponse.from_response(
            await self.run_one(
                "merge",
                _to_request(
                    documents=documents,
                    use_def_eq=use_def_eq,
                    include_alts_as_comments=include_alts_as_comments,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def theorem2sorry(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> Theorem2SorryResponse:
        """Replace proofs with sorry."""
        return Theorem2SorryResponse.from_response(
            await self.run_one(
                "theorem2sorry",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def rename(
        self,
        content: str,
        declarations: dict[str, str],
        environment: str,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> RenameResponse:
        """Rename declarations in code."""
        return RenameResponse.from_response(
            await self.run_one(
                "rename",
                _to_request(
                    content=content,
                    declarations=declarations,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def theorem2lemma(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        target: str | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> Theorem2LemmaResponse:
        """Convert theorem/lemma keywords."""
        return Theorem2LemmaResponse.from_response(
            await self.run_one(
                "theorem2lemma",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    target=target,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def check(
        self,
        content: str,
        environment: str,
        mathlib_options: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> CheckResponse:
        """Evaluate Lean code for errors."""
        return CheckResponse.from_response(
            await self.run_one(
                "check",
                _to_request(
                    content=content,
                    mathlib_options=mathlib_options,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def simplify_theorems(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        simplifications: list[str] | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> SimplifyTheoremsResponse:
        """Simplify theorem proofs."""
        return SimplifyTheoremsResponse.from_response(
            await self.run_one(
                "simplify_theorems",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    simplifications=simplifications,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def repair_proofs(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        repairs: list[str] | None = None,
        terminal_tactics: list[str] | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> RepairProofsResponse:
        """Repair broken proofs."""
        return RepairProofsResponse.from_response(
            await self.run_one(
                "repair_proofs",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    repairs=repairs,
                    terminal_tactics=terminal_tactics,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def have2lemma(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        include_have_body: bool | None = None,
        include_whole_context: bool | None = None,
        reconstruct_callsite: bool | None = None,
        verbosity: int | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> Have2LemmaResponse:
        """Extract have statements to lemmas."""
        return Have2LemmaResponse.from_response(
            await self.run_one(
                "have2lemma",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    include_have_body=include_have_body,
                    include_whole_context=include_whole_context,
                    reconstruct_callsite=reconstruct_callsite,
                    verbosity=verbosity,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def have2sorry(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> Have2SorryResponse:
        """Replace have statements with sorry."""
        return Have2SorryResponse.from_response(
            await self.run_one(
                "have2sorry",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def sorry2lemma(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        extract_sorries: bool | None = None,
        extract_errors: bool | None = None,
        include_whole_context: bool | None = None,
        reconstruct_callsite: bool | None = None,
        merge_duplicates: bool | None = None,
        theorems_only: bool | None = None,
        verbosity: int | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> Sorry2LemmaResponse:
        """Extract sorries and errors to standalone lemmas."""
        return Sorry2LemmaResponse.from_response(
            await self.run_one(
                "sorry2lemma",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    extract_sorries=extract_sorries,
                    extract_errors=extract_errors,
                    include_whole_context=include_whole_context,
                    reconstruct_callsite=reconstruct_callsite,
                    merge_duplicates=merge_duplicates,
                    theorems_only=theorems_only,
                    verbosity=verbosity,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def disprove(
        self,
        content: str,
        environment: str,
        names: list[str] | None = None,
        indices: list[int] | None = None,
        terminal_tactics: list[str] | None = None,
        theorems_only: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> DisproveResponse:
        """Attempt to disprove theorems."""
        return DisproveResponse.from_response(
            await self.run_one(
                "disprove",
                _to_request(
                    content=content,
                    names=names,
                    indices=indices,
                    terminal_tactics=terminal_tactics,
                    theorems_only=theorems_only,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def normalize(
        self,
        content: str,
        environment: str,
        normalizations: list[str] | None = None,
        failsafe: bool | None = None,
        ignore_imports: bool | None = None,
        timeout_seconds: float | None = None,
    ) -> NormalizeResponse:
        """Standardize Lean formatting."""
        return NormalizeResponse.from_response(
            await self.run_one(
                "normalize",
                _to_request(
                    content=content,
                    normalizations=normalizations,
                    failsafe=failsafe,
                    ignore_imports=ignore_imports,
                    environment=environment,
                    timeout_seconds=timeout_seconds,
                ),
            )
        )

    async def environments(self, timeout_seconds: float | None = None) -> list[dict[str, Any]]:
        """Retrieve the list of available environments."""
        response_text = await self._call(
            "v1/environments",
            timeout_seconds,
            http_method=HTTPMethod.GET,
        )
        return cast(list[dict[str, Any]], json.loads(response_text))

    async def prove_riemann(self, timeout_seconds: float | None = None) -> JsonDict:
        response_text = await self._call(
            "v1/prove_riemann",
            timeout_seconds,
            http_method=HTTPMethod.GET,
        )
        return cast(JsonDict, json.loads(response_text))

    # Implementation details:

    def _get_session(self) -> aiohttp.ClientSession | httpx.AsyncClient:
        if self._http2:
            return httpx.AsyncClient(
                http2=True,
                limits=httpx.Limits(
                    max_connections=self.max_concurrency,
                    max_keepalive_connections=self.max_concurrency,
                    keepalive_expiry=120,
                ),
                headers=self._headers,
                trust_env=True,
            )
        connector = aiohttp.TCPConnector(
            limit=self.max_concurrency,
            limit_per_host=self.max_concurrency,
            force_close=False,
            keepalive_timeout=120,  # default 15s causes connection churn
            ttl_dns_cache=300,  # 5-min DNS cache; default 10s causes resolver pressure under load
        )
        return aiohttp.ClientSession(
            trust_env=True,
            connector=connector,
            headers=self._headers,
        )

    def _session_closed(self) -> bool:
        """Transport-agnostic 'is session closed?' check."""
        if self._session is None:
            return True
        if self._http2:
            return bool(cast(httpx.AsyncClient, self._session).is_closed)
        return bool(cast(aiohttp.ClientSession, self._session).closed)

    async def _rotate_session(self, stale: aiohttp.ClientSession | httpx.AsyncClient) -> None:
        """Drop the cached session if it is the one that just failed, so the
        next attempt rebuilds a fresh one."""
        async with self._session_lock:
            if self._session is stale:
                self._session = None
        try:
            if isinstance(stale, httpx.AsyncClient):
                await stale.aclose()
            else:
                await stale.close()
        except Exception:
            logger.debug("error closing stale session", exc_info=True)

    async def _rotate_and_refetch_httpx(self, stale: httpx.AsyncClient) -> httpx.AsyncClient:
        """Rotate the session on a failed httpx client and return a fresh one
        for the caller's next inline attempt."""
        await self._rotate_session(stale)
        async with self._session_lock:
            if self._session_closed():
                self._session = self._get_session()
            return cast(httpx.AsyncClient, self._session)

    async def _call(
        self,
        method: str,
        request_timeout_seconds: float | None,
        http_method: HTTPMethod = HTTPMethod.POST,
        data: str | None = None,
    ) -> str:
        # Figure out how long to wait for the server to produce a response. We
        # add request timeout to base timeout (e.g., 600s request + 1800s base =
        # 2400s total client wait)
        call_timeout_seconds = self.base_timeout_seconds + (request_timeout_seconds or 0)

        # Retry transient errors with exponential backoff:
        # - AxleIsUnavailable: connection errors, 503 Service Unavailable
        # - AxleRateLimitedError: 429 Too Many Requests
        @retry(
            retry=retry_if_exception_type((AxleIsUnavailable, AxleRateLimitedError)),
            wait=wait_exponential_jitter(initial=1, max=15),
            stop=stop_after_delay(call_timeout_seconds),
            reraise=True,
            before_sleep=before_sleep_log(logger, logging.WARNING),
        )
        async def _call_with_retry() -> str:
            return await self._call_attempt(method, call_timeout_seconds, http_method, data)

        return cast(str, await _call_with_retry())

    async def _call_attempt(
        self,
        method: str,
        call_timeout_seconds: float,
        http_method: HTTPMethod,
        data: str | None,
    ) -> str:
        async with self._session_lock:
            if self._session_closed():
                self._session = self._get_session()

        url = f"{self.url}/{method}"

        # Gate the actual request so `max_concurrency` caps concurrent in-flight requests.
        async with self._sem:
            if self._http2:
                return await self._call_attempt_httpx(
                    cast(httpx.AsyncClient, self._session),
                    url,
                    call_timeout_seconds,
                    http_method,
                    data,
                )
            return await self._call_attempt_aiohttp(
                cast(aiohttp.ClientSession, self._session),
                url,
                call_timeout_seconds,
                http_method,
                data,
            )

    async def _call_attempt_aiohttp(
        self,
        session: aiohttp.ClientSession,
        url: str,
        call_timeout_seconds: float,
        http_method: HTTPMethod,
        data: str | None,
    ) -> str:
        try:
            timeout = aiohttp.ClientTimeout(total=call_timeout_seconds)
            if http_method == HTTPMethod.GET:
                response = await session.get(url, timeout=timeout)
            else:
                if data is None:
                    raise ValueError(f"data parameter is required for {http_method} requests")
                response = await session.post(url, data=data, timeout=timeout)
            await self._raise_for_status_aiohttp(response)
            return cast(str, await response.text())
        except aiohttp.ClientConnectionError as e:
            raise AxleIsUnavailable(self.url, _exc_msg(e)) from e
        except RuntimeError as e:
            if "Connection closed." in str(e) or "Session is closed" in str(e):
                raise AxleIsUnavailable(self.url, _exc_msg(e)) from e
            raise
        except TimeoutError:
            raise AxleIsUnavailable(
                self.url, f"client timeout: server did not respond within {call_timeout_seconds}s"
            ) from None

    async def _call_attempt_httpx(
        self,
        client: httpx.AsyncClient,
        url: str,
        call_timeout_seconds: float,
        http_method: HTTPMethod,
        data: str | None,
    ) -> str:
        timeout = httpx.Timeout(call_timeout_seconds)
        # Retry HTTP/2 GOAWAY inline so it bypasses the outer exponential backoff.
        last_goaway: httpx.RemoteProtocolError | None = None
        for _ in range(_GOAWAY_RETRY_LIMIT):
            try:
                if http_method == HTTPMethod.GET:
                    response = await client.get(url, timeout=timeout)
                else:
                    if data is None:
                        raise ValueError(f"data parameter is required for {http_method} requests")
                    response = await client.post(url, content=data, timeout=timeout)
            except httpx.TimeoutException as e:
                raise AxleIsUnavailable(
                    self.url,
                    f"client timeout: server did not respond within {call_timeout_seconds}s",
                ) from e
            except httpx.RemoteProtocolError as e:
                if _is_graceful_goaway(e):
                    # Evict the dead connection from the pool
                    logger.debug("HTTP/2 GOAWAY received; rotating client")
                    client = await self._rotate_and_refetch_httpx(client)
                    last_goaway = e
                    continue
                raise AxleIsUnavailable(self.url, _exc_msg(e)) from e
            except httpx.LocalProtocolError as e:
                # httpcore leaks h2 stream slots on aborted streams.
                # Rotate the cached session and let outer tenacity retry on
                # a fresh client.
                if not _is_stream_limit_error(e):
                    raise
                logger.warning(
                    "HTTP/2 stream limit reached on cached connection; rotating client. %s",
                    e,
                )
                await self._rotate_session(client)
                raise AxleIsUnavailable(self.url, _exc_msg(e)) from e
            except (httpx.ConnectError, httpx.NetworkError) as e:
                raise AxleIsUnavailable(self.url, _exc_msg(e)) from e
            except RuntimeError as e:
                if "client has been closed" in str(e):
                    raise AxleIsUnavailable(self.url, _exc_msg(e)) from e
                raise
            self._raise_for_status_httpx(response)
            return cast(str, response.text)
        assert last_goaway is not None
        raise AxleIsUnavailable(self.url, _exc_msg(last_goaway)) from last_goaway

    async def _raise_for_status_aiohttp(self, response: aiohttp.ClientResponse) -> None:
        if response.status == 200:
            return
        error_message = await response.text()
        self._raise_for_status_code(response.status, error_message)

    def _raise_for_status_httpx(self, response: httpx.Response) -> None:
        if response.status_code == 200:
            return
        self._raise_for_status_code(response.status_code, response.text)

    def _raise_for_status_code(self, status: int, error_message: str) -> None:
        match status:
            # Retryable errors
            case 429:
                raise AxleRateLimitedError(f"Rate limited: {error_message}")
            case 503:
                raise AxleIsUnavailable(self.url, f"Service unavailable: {error_message}")

            # Non-retryable client errors
            case 400:
                raise AxleInvalidArgument(f"Bad request: {error_message}")
            case 403:
                raise AxleForbiddenError(f"Forbidden: {error_message}")
            case 404:
                raise AxleNotFoundError(f"Not found: {error_message}")
            case 409:
                raise AxleConflictError(f"Conflict: {error_message}")

            # Catch-all for other errors
            case 500:
                raise AxleInternalError(f"Internal server error: {error_message}")
            case _ if 400 <= status < 500:
                raise AxleInvalidArgument(f"Client error {status}: {error_message}")
            case _:
                raise AxleInternalError(f"Server error {status}: {error_message}")

    async def close(self) -> None:
        """Close the client session and cleanup resources."""
        logger.debug("Closing AXLE session")
        if self._session is None:
            return
        if self._http2:
            session_h2 = cast(httpx.AsyncClient, self._session)
            if not session_h2.is_closed:
                await session_h2.aclose()
        else:
            session_h1 = cast(aiohttp.ClientSession, self._session)
            if not session_h1.closed:
                await session_h1.close()
        self._session = None

    async def __aenter__(self) -> "AxleClient":
        """Enter async context manager."""
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Exit async context manager, closing the session."""
        await self.close()


def _to_request(**kwargs: Any) -> JsonDict:
    return {key: value for key, value in kwargs.items() if value is not None}


# Bound on inline GOAWAY retries
_GOAWAY_RETRY_LIMIT: Final[int] = 3


def _exc_msg(exc: BaseException) -> str:
    """str(exc) if it carries info, otherwise the exception type."""
    msg = str(exc)
    return msg if msg else type(exc).__name__


def _is_graceful_goaway(exc: httpx.RemoteProtocolError) -> bool:
    """True if `exc` represents an HTTP/2 GOAWAY with NO_ERROR (graceful shutdown).

    h2's `ConnectionTerminated` repr is
    `<ConnectionTerminated error_code:0, last_stream_id:..., additional_data:...>`,
    which httpx surfaces as the message body of `RemoteProtocolError`.
    """
    msg = str(exc)
    return "ConnectionTerminated" in msg and "error_code:0" in msg


def _is_stream_limit_error(exc: httpx.LocalProtocolError) -> bool:
    """True if `exc` is the h2 stream-limit cliff caused by httpcore's stream-
    slot leak on aborted streams.

    h2 raises `TooManyStreamsError("Max outbound streams is N, M open")` which
    httpx surfaces as `LocalProtocolError`.
    """
    return "Max outbound streams" in str(exc)
