#!/bin/bash
set -e

# Configuration
DOCS_SRC="docs/src"
WIKI_DIR="wiki-export"
REPO_URL="https://github.com/copyleftdev/oxide-ci.wiki.git"

# Prepare export directory
rm -rf "$WIKI_DIR"
mkdir -p "$WIKI_DIR"
cd "$WIKI_DIR"
git init
git branch -m master
git remote add origin "$REPO_URL"

# Copy files
echo "Copying files..."
cp ../$DOCS_SRC/*.md .

# Transform Home
if [ -f "intro.md" ]; then
    mv intro.md Home.md
fi

# Transform Sidebar
if [ -f "SUMMARY.md" ]; then
    mv SUMMARY.md _Sidebar.md
    # Simple transform: Remove .md extensions from links, as Wiki serves them at root
    # mdBook: [Link](file.md) -> Wiki: [Link](file)
    # Also indentation might need adjustment, but standard lists are fine.
    sed -i 's/(\(.*\)\.md)/(\1)/g' _Sidebar.md
fi

# Transform Links in all files
# Remove .md extension from relative links
# (Simple regex, might be brittle but works for this structure)
find . -name "*.md" -print0 | xargs -0 sed -i 's/(\(.*\)\.md)/(\1)/g'

# Commit
git add .
git commit -m "Sync documentation from oxide-ci main repo"

echo "Ready to push. If Wiki is initialized, run: cd $WIKI_DIR && git push -f origin master"
