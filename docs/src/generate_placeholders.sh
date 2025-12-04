#!/bin/bash
# Generate placeholder files for all pages referenced in SUMMARY.md

PAGES=(
  "installation.md"
  "prerequisites.md"
  "quickstart.md"
  "configuration.md"
  "env-vars.md"
  "authentication.md"
  "api-overview.md"
  "health-status.md"
  "zone-operations.md"
  "creating-zones.md"
  "zone-config.md"
  "dns-records.md"
  "managing-zones.md"
  "reloading-zones.md"
  "deleting-zones.md"
  "zone-status.md"
  "deployment.md"
  "docker.md"
  "kubernetes.md"
  "monitoring.md"
  "logging.md"
  "troubleshooting.md"
  "dev-setup.md"
  "building.md"
  "testing.md"
  "architecture.md"
  "api-design.md"
  "rndc-integration.md"
  "contributing.md"
  "api-reference.md"
  "api-health.md"
  "api-zones.md"
  "api-status-codes.md"
  "examples.md"
)

for page in "${PAGES[@]}"; do
  if [ ! -f "$page" ]; then
    title=$(basename "$page" .md | sed 's/-/ /g' | awk '{for(i=1;i<=NF;i++) $i=toupper(substr($i,1,1)) substr($i,2)}1')
    echo "# $title" > "$page"
    echo "" >> "$page"
    echo "This page is under construction." >> "$page"
    echo "Created placeholder: $page"
  fi
done

echo "Done! All placeholder files have been created."
