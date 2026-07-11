#!/usr/bin/env python3
"""A minimal gieter external emitter: renders the catalog as a Markdown list.

  stdin  <- EmitRequest  { ir_version, catalog, options }
  stdout -> EmitResponse { ir_version, files: [{path, contents}], warnings }
"""

import json
import sys

IR_VERSION = 1


def type_name(column_type):
    kind = column_type["kind"]
    if kind == "scalar":
        return column_type["value"]["kind"]
    if kind == "array":
        return type_name(column_type["value"]) + "[]"
    reference = column_type["value"]  # enum / composite / domain
    return f"{reference['schema']}.{reference['name']}"


def main():
    request = json.load(sys.stdin)

    lines = ["# Catalog", ""]
    for schema in request["catalog"]["schemas"]:
        for table in schema["tables"]:
            lines.append(f"## {schema['name']}.{table['name']}")
            for column in table["columns"]:
                suffix = "" if column["nullable"] else " not null"
                lines.append(f"- `{column['name']}`: {type_name(column['ty'])}{suffix}")
            for fk in table["foreign_keys"]:
                target = fk["ref_table"]
                lines.append(f"- fk `{', '.join(fk['columns'])}` -> {target[0]}.{target[1]}")
            lines.append("")

    response = {
        "ir_version": IR_VERSION,
        "files": [{"path": "schema.md", "contents": "\n".join(lines)}],
        "warnings": [],
    }
    json.dump(response, sys.stdout)


if __name__ == "__main__":
    main()
