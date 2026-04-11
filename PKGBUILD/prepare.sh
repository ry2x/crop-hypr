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

read_changelog() {
	local changelog_arg=${1:-}

	if [ -n "$changelog_arg" ]; then
		printf '%s\n' "$changelog_arg"
		return
	fi

	read -r -p "What is the changelog summary for this release?: " input_changelog
	printf '%s\n' "$input_changelog"
}

validate_version() {
	local version=$1
	if [ -z "$version" ]; then
		echo "Error: version cannot be empty" >&2
		exit 1
	fi
}

validate_changelog() {
	local changelog=$1
	if [ -z "$changelog" ]; then
		echo "Error: changelog cannot be empty" >&2
		exit 1
	fi
}

escape_sed_replacement() {
	printf '%s\n' "$1" | sed 's/[\/&|\\]/\\&/g'
}

update_pkgbuild_files() {
	local module_path=$1
	local version=$2
	local escaped_version

	escaped_version=$(escape_sed_replacement "$version")

	pushd "$ROOT_DIR/$module_path" >/dev/null
	sed -i "s|^pkgver=.*|pkgver=$escaped_version|" PKGBUILD
	sed -i "s|^pkgrel=.*|pkgrel=1|" PKGBUILD
	updpkgsums
	makepkg --printsrcinfo > .SRCINFO
	popd >/dev/null
}

commit_and_push_submodule() {
	local module_path=$1
	local version=$2
	local changelog=$3

	pushd "$ROOT_DIR/$module_path" >/dev/null
	git checkout master
	git add PKGBUILD .SRCINFO

	if ! git diff --cached --quiet; then
		git commit -m "Update to version $version" -m "Changelog: $changelog"
		git push origin master
	else
		echo "No changes to commit in $module_path"
	fi

	popd >/dev/null
}

commit_and_push_main_repo() {
	local version=$1
	local changelog=$2

	pushd "$ROOT_DIR" >/dev/null
	git add "${SUBMODULE_DIRS[@]}"

	if ! git diff --cached --quiet; then
		git commit -m "🧹 CHORE: Update PKGBUILD submodules to version $version" -m "Changelog: $changelog"
		git push
	else
		echo "No changes to commit in main repository"
	fi

	popd >/dev/null
}

main() {
	require_commands

	local version
	local changelog
	version=$(read_version "${1:-}")
	changelog=$(read_changelog "${2:-}")
	validate_version "$version"
	validate_changelog "$changelog"

	git -C "$ROOT_DIR" submodule update --init --recursive

	local module
	for module in "${SUBMODULE_DIRS[@]}"; do
		update_pkgbuild_files "$module" "$version"
	done

	for module in "${SUBMODULE_DIRS[@]}"; do
		commit_and_push_submodule "$module" "$version" "$changelog"
	done

	commit_and_push_main_repo "$version" "$changelog"
}

main "${1:-}"
