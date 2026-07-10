-- All enum types with their ordered values, in the target schemas.
-- $1 = schema names (text[])
SELECT
    n.nspname AS schema,
    t.typname AS name,
    array_agg(
        e.enumlabel
        ORDER BY
        e.enumsortorder
    ) AS "values"
FROM
    pg_type t
    JOIN pg_enum e ON e.enumtypid = t.oid
    JOIN pg_namespace n ON n.oid = t.typnamespace
WHERE
    n.nspname = ANY ($1)
GROUP BY
    n.nspname,
    t.typname;