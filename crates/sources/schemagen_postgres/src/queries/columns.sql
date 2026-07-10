-- All columns of all relations (tables and views) in the target schemas.
-- The caller groups rows by (schema, table_name) and maps udt/typtype/typmod onto ScalarType.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    c.relname AS table_name,
    a.attname AS name,
    a.attnum AS ordinal,
    NOT a.attnotnull AS nullable,
    a.atttypmod AS typmod, -- numeric precision/scale, varchar/char length, time precision
    t.typname AS udt, -- int4, text, _int4 (array), mood (enum)
    t.typtype AS typtype, -- 'e'=enum, 'b'=base, 'c'=composite, 'd'=domain
    et.typname AS elem_udt, -- element udt when t is an array
    ett.typtype AS elem_typtype,
    tn.nspname AS type_schema,
    col_description(a.attrelid, a.attnum) AS comment
FROM
    pg_attribute a
    JOIN pg_class c ON c.oid = a.attrelid
    JOIN pg_namespace n ON n.oid = c.relnamespace
    JOIN pg_type t ON t.oid = a.atttypid
    JOIN pg_namespace tn ON tn.oid = t.typnamespace
    LEFT JOIN pg_type et ON et.oid = t.typelem
    AND t.typcategory = 'A'
    LEFT JOIN pg_type ett ON ett.oid = et.oid
WHERE
    n.nspname = ANY ($1)
    AND c.relkind IN ('r', 'p', 'v', 'm')
    AND a.attnum > 0
    AND NOT a.attisdropped
ORDER BY
    n.nspname,
    c.relname,
    a.attname;