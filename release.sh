#!/bin/bash

tasks=("Updated CHANGELOG.md" "Updated README.md" "Merged into 'main'")
status=()
for _ in "${tasks[@]}"; do status+=(" "); done

BOLD='\033[1m'
BLUE='\033[1;34m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

add_git_release() {
    local version=$1

    if [ -z "$version" ]; then
        echo "Error: You must provide a version tag (e.g., v1.0.0)"
        return 1
    fi

    echo "Preparing git release: $version"

    git switch main || return 1
    git tag "$version" || return 1
    git push origin "$version" || return 1

    echo "Successfully pushed tag $version to origin."
}

for i in "${!tasks[@]}"; do
    clear
    
    if [ "$i" -gt 0 ]; then
        echo -e "${BOLD}HISTORY:${NC}"
        for j in $(seq 0 $((i-1))); do
            if [[ "${status[$j]}" == "x" ]]; then
                echo -e "${GREEN}[x] ${tasks[$j]}${NC}"
            else
                echo -e "${RED}[ ] ${tasks[$j]}${NC}"
            fi
        done
        echo "----------------------------------------"
    fi

    echo -e "${BLUE}[?] ${tasks[$i]}${NC}"
    echo "----------------------------------------"
    
    read -p "Did you finish this? (y/n): " answer
    
    if [[ "$answer" =~ ^[Yy]$ ]]; then
        status[$i]="x"
    fi
done

clear


all_done=true
for i in "${!status[@]}"; do
    [[ "${status[$i]}" != "x" ]] && all_done=false
done

if [ "$all_done" = true ]; then
    read -p "Enter version tag (e.g., v1.0.0): " version_tag
    add_git_release "$version_tag"
else
    echo -e "${RED}You still have some pending tasks. Keep going!${NC}"
fi

