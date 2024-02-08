$DB_USER = "user"
$DB_PASSWORD = "password"
$DB_NAME = "newsletter"
$DB_HOST = "localhost"
$DB_PORT = "5432"
$DB_URL = "postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

Write-Host "Creating database..."
Invoke-Expression "sqlx database create --database-url $DB_URL"
Write-Host "Database created"
Write-Host "Running migrations..."
Invoke-Expression "sqlx migrate run --database-url $DB_URL"

Write-Host "Finished migrations"
