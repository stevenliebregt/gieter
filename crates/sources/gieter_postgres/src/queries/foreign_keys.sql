-- All foreign keys in the target schemas, with local and referenced column lists in order.
-- The caller groups rows by (schema, table_name) and attaches them to the owning Table.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    lc.relname AS table_name,
    con.conname AS name,
    (
        SELECT
            array_agg(
                att.attname
                ORDER BY
                k.ord
            )
        FROM
            unnest(con.conkey) WITH ORDINALITY AS k (attnum, ord)
            JOIN pg_attribute att ON att.attrelid = con.conrelid AND att.attnum = k.attnum
    ) AS local_columns,
    fn.nspname AS ref_schema,
    fc.relname AS ref_table,
    (
        SELECT
            array_agg(
                att.attname
                ORDER BY
                k.ord
            )
        FROM
            unnest(con.confkey) WITH ORDINALITY AS k (attnum, ord)
            JOIN pg_attribute att ON att.attrelid = con.confrelid AND att.attnum = k.attnum
    ) AS ref_columns
FROM
    pg_constraint con
    JOIN pg_class lc ON lc.oid = con.conrelid
    JOIN pg_namespace n ON n.oid = lc.relnamespace
    JOIN pg_class fc ON fc.oid = con.confrelid
    JOIN pg_namespace fn ON fn.oid = fc.relnamespace
WHERE
    n.nspname = ANY ($1)
    AND con.contype = 'f';