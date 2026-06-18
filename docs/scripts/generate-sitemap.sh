#!/bin/bash
set -euo pipefail

BASE_URL="https://lazyxeon.github.io/AstraWeave"
BOOK_DIR="${1:-docs/book}"
OUTPUT_FILE="${BOOK_DIR}/sitemap.xml"
PRIORITY_HIGH="1.0"
PRIORITY_MEDIUM="0.8"
PRIORITY_LOW="0.6"
TODAY=$(date -u +"%Y-%m-%d")

cat > "$OUTPUT_FILE" << 'HEADER'
<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"
        xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
        xsi:schemaLocation="http://www.sitemaps.org/schemas/sitemap/0.9
        http://www.sitemaps.org/schemas/sitemap/0.9/sitemap.xsd">
HEADER

add_url() {
    local url="$1"
    local priority="$2"
    local changefreq="${3:-weekly}"
    cat >> "$OUTPUT_FILE" << URL
  <url>
    <loc>${url}</loc>
    <lastmod>${TODAY}</lastmod>
    <changefreq>${changefreq}</changefreq>
    <priority>${priority}</priority>
  </url>
URL
}

add_url "${BASE_URL}/" "$PRIORITY_HIGH" "daily"

find "$BOOK_DIR" -name "*.html" -type f | while read -r file; do
    rel_path="${file#$BOOK_DIR/}"
    
    [[ "$rel_path" == "print.html" ]] && continue
    [[ "$rel_path" == "404.html" ]] && continue
    [[ "$rel_path" == api/src/* ]] && continue
    [[ "$rel_path" == api/implementors/* ]] && continue
    
    url="${BASE_URL}/${rel_path}"
    
    case "$rel_path" in
        index.html)
            continue
            ;;
        getting-started/*.html|architecture/*.html)
            priority="$PRIORITY_HIGH"
            ;;
        core-systems/*.html|api/*.html)
            priority="$PRIORITY_MEDIUM"
            ;;
        *)
            priority="$PRIORITY_LOW"
            ;;
    esac
    
    add_url "$url" "$priority"
done

echo '</urlset>' >> "$OUTPUT_FILE"

echo "Sitemap generated at ${OUTPUT_FILE}"
echo "Total URLs: $(grep -c '<url>' "$OUTPUT_FILE")"
