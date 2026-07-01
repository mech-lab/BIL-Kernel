#!/usr/bin/env python3
"""Extract theorems from a file and validate each one."""

import asyncio
import textwrap

from axle import AxleClient
from axle.exceptions import AxleApiError


async def main() -> None:
    """Extract and validate theorems."""
    combined_file = """
import Mathlib

/-- Helper lemma -/
lemma helper : 1 = 1 := rfl

/-- Main theorem using helper -/
theorem main_result : 1 + 0 = 1 := by
  simp [helper]

/-- Another theorem -/
theorem another : 2 = 2 := rfl
"""

    try:
        async with AxleClient() as client:
            # Extract theorems
            print("Extracting theorems...")
            extract_result = await client.extract_theorems(
                content=combined_file, environment="lean-4.28.0"
            )

            if extract_result.lean_messages.errors:
                print("Extraction had errors:")
                for msg in extract_result.lean_messages.errors:
                    print(f"  - {msg}")
                return

            print(f"Found {len(extract_result.documents)} theorems:\n")

            # Process each theorem
            for name, doc in extract_result.documents.items():
                print(f"Theorem: {name}")
                print(f"  Signature:\n{textwrap.indent(doc.signature, '    ')}")
                print(
                    f"  Dependencies: {doc.local_type_dependencies + doc.local_value_dependencies}"
                )

                # okay means it compiled; failed_declarations catches sorry, disallowed axioms, etc. that leave okay true.
                check_result = await client.check(content=doc.content, environment="lean-4.28.0")
                valid = check_result.okay and not check_result.failed_declarations
                status = "VALID" if valid else "INVALID"
                print(f"  Status: {status}")
                print()
    except AxleApiError as e:
        print(f"API error: {e}")


if __name__ == "__main__":
    asyncio.run(main())
