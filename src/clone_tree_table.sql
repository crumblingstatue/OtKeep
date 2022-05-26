INSERT INTO tree_scripts
SELECT
    :dst,
    blob_id,
    name,
    desc
FROM tree_scripts
WHERE tree_id = :src;

INSERT INTO tree_files
SELECT
    :dst,
    blob_id,
    name,
    desc
FROM tree_files
WHERE tree_id = :src;