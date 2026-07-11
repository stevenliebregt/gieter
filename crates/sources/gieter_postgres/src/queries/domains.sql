-- Domain types (CREATE DOMAIN) in the target schemas, with their base type described
-- the same way columns.sql describes a column's type so the caller can reuse resolve_type.
-- typtypmod carries the modifier applied to the base (e.g. a domain over varchar(10)).
-- checks are the raw CHECK constraint texts.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    t.typname AS name,
    t.typnotnull AS not_null,
    t.typdefault AS "default",
    bt.typname AS base_udt,
    bt.typtype::text AS base_typtype,
    t.typtypmod AS base_typmod,
    et.typname AS base_elem_udt,
    ett.typtype::text AS base_elem_typtype,
    btn.nspname AS base_type_schema,
    COALESCE(
        (
            SELECT
                array_agg(
                    pg_get_constraintdef(con.oid)
                    ORDER BY
                        con.conname
                )
            FROM
                pg_constraint con
            WHERE
                con.contypid = t.oid
                AND con.contype = 'c'
        ),
        '{}'
    ) AS checks
FROM
    pg_type t
    JOIN pg_namespace n ON n.oid = t.typnamespace
    JOIN pg_type bt ON bt.oid = t.typbasetype
    JOIN pg_namespace btn ON btn.oid = bt.typnamespace
    LEFT JOIN pg_type et ON et.oid = bt.typelem
    AND bt.typcategory = 'A'
    LEFT JOIN pg_type ett ON ett.oid = et.oid
WHERE
    n.nspname = ANY ($1)
    AND t.typtype = 'd';
