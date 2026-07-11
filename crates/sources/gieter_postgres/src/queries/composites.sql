-- Fields of standalone composite types (CREATE TYPE ... AS (...)) in the target schemas.
-- Shaped exactly like columns.sql so the caller can reuse ColumnRow + column_type;
-- table_name carries the composite type name. Composite attributes cannot be NOT NULL,
-- so nullable always comes back true.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    c.relname AS table_name,
    a.attname AS name,
    a.attnum AS ordinal,
    NOT a.attnotnull AS nullable,
    a.atttypmod AS typmod,
    t.typname AS udt,
    t.typtype::text AS typtype,
    et.typname AS elem_udt,
    ett.typtype::text AS elem_typtype,
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
    AND c.relkind = 'c'
    AND a.attnum > 0
    AND NOT a.attisdropped;
