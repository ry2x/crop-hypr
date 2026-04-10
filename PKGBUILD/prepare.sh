#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(git rev-parse --show-toplevel)
SUBMODULE_DIRS=(
	"PKGBUILD/hyprcrop"
	"PKGBUILD/hyprcrop-bin"
)

require_commands() {
	local cmd
	for cmd in git sed updpkgsums makepkg; do
		if ! command -v "$cmd" >/dev/null 2>&1; then
			echo "Error: required command not found: $cmd" >&2
			exit 1
		fi
	done
}

read_version() {
	local version_arg=${1:-}

	if [ -n "$version_arg" ]; then
		printf '%s\n' "$version_arg"
		return
	fi

	read -r -p "What is the new version number? (not including the 'v' prefix): " input_version
	printf '%s\n' "$input_version"
}

validate_version() {
	local version=$1
	if [ -z "$version" ]; then
		echo "Error: version cannot be empty" >&2
		exit 1
	fi
}

update_pkgbuild_files() {
	local module_path=$1

	pushd "$ROOT_DIR/$module_path" >/dev/null
	sed -i "s/pkgver=.*/pkgver=$VERSION/" PKGBUILD
	updpkgsums
	makepkg --printsrcinfo > .SRCINFO
	popd >/dev/null
}

commit_and_push_submodule() {
	local module_path=$1

	pushd "$ROOT_DIR/$module_path" >/dev/null
	git add PKGBUILD .SRCINFO

	if ! git diff --cached --quiet; then
		git commit -m "Update to version $VERSION"
		git push
	else
		echo "No changes to commit in $module_path"
	fi

	popd >/dev/null
}

commit_and_push_main_repo() {
	pushd "$ROOT_DIR" >/dev/null
	git add PKGBUILD

	if ! git diff --cached --quiet; then
		git commit -m "🧹 CHORE: Update PKGBUILD submodules to version $VERSION"
		git push
	else
		echo "No changes to commit in main repository"
	fi

	popd >/dev/null
}

main() {
	require_commands

	VERSION=$(read_version "${1:-}")
	validate_version "$VERSION"

	git -C "$ROOT_DIR" submodule update --init --recursive

	local module
	for module in "${SUBMODULE_DIRS[@]}"; do
		update_pkgbuild_files "$module"
	done

	for module in "${SUBMODULE_DIRS[@]}"; do
		commit_and_push_submodule "$module"
	done

	commit_and_push_main_repo
}

main "${1:-}"
