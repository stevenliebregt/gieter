#!/usr/bin/env python3
"""A minimal gieter external source: emits a fixed, made-up catalog (no database).

  stdin  <- SourceRequest  { ir_version, schemas, options }
  stdout -> SourceResponse { ir_version, catalog }
"""

import json
import sys

IR_VERSION = 1


def scalar(kind, value=None):
    inner = {"kind": kind}
    if value is not None:
        inner["value"] = value
    return {"kind": "scalar", "value": inner}


def column(name, ty, nullable=False):
    return {"name": name, "ty": ty, "nullable": nullable, "comment": None}


def table(name, columns, foreign_keys=None):
    return {
        "name": name,
        "schema": "public",
        "columns": columns,
        "primary_key": ["id"],
        "foreign_keys": foreign_keys or [],
        "comment": None,
    }


def main():
    json.load(sys.stdin)  # the request is ignored; this source is hardcoded

    # In a real world emitter you would most probably connect to your datasource here and introspect it. Then
    # build up the catalog to return it as a response.

    authors = table(
        "authors",
        [column("id", scalar("int64")), column("name", scalar("text", {"max_len": None}))],
    )
    books = table(
        "books",
        [
            column("id", scalar("int64")),
            column("title", scalar("text", {"max_len": None})),
            column("author_id", scalar("int64")),
        ],
        foreign_keys=[
            {
                "name": "books_author_id_fkey",
                "columns": ["author_id"],
                "ref_table": ["public", "authors"],
                "ref_columns": ["id"],
            }
        ],
    )

    catalog = {
        "schemas": [
            {
                "name": "public",
                "tables": [authors, books],
                "enums": [],
                "views": [],
                "composites": [],
                "domains": [],
            }
        ]
    }

    json.dump({"ir_version": IR_VERSION, "catalog": catalog}, sys.stdout)


if __name__ == "__main__":
    main()
