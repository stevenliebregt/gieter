-- Primary keys of all tables in the target schemas, with their columns in key order.
-- The caller groups rows by (schema, table_name) and attaches them to the owning Table.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    c.relname AS table_name,
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
    ) AS columns
FROM
    pg_constraint con
    JOIN pg_class c ON c.oid = con.conrelid
    JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE
    n.nspname = ANY ($1)
    AND con.contype = 'p';
