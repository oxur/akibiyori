#!/bin/sh
set -eu

usage() {
  printf '%s\n' "usage: $0 root|prefix OUTPUT" >&2
  exit 2
}

[ "$#" -eq 2 ] || usage
mode=$1
requested_output=$2
case "$mode" in
  root) base_path='' ;;
  prefix) base_path='/akibiyori-docs' ;;
  *) usage ;;
esac
[ -n "$requested_output" ] || usage
case "$requested_output" in
  /|.|..|*/.|*/./*|*/..|../*|*/../*)
    printf 'refusing unsafe output path: %s\n' "$requested_output" >&2
    exit 1
    ;;
esac

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
case "$requested_output" in
  /*) lexical_output=$requested_output ;;
  *) lexical_output=$(pwd -P)/$requested_output ;;
esac

guard_output_namespace() {
  guard_candidate=$1
  case "$guard_candidate" in
    "$repo_root"/target/demo-site/root|"$repo_root"/target/demo-site/prefix) ;;
    "$repo_root"/*)
      printf 'refusing output inside the source checkout: %s\n' "$guard_candidate" >&2
      return 1
      ;;
  esac
  case "$repo_root" in
    "$guard_candidate"/*)
      printf 'refusing output above the source checkout: %s\n' "$guard_candidate" >&2
      return 1
      ;;
  esac
  case "$guard_candidate" in
    /|"$repo_root"|"$HOME")
      printf 'refusing unsafe output path: %s\n' "$guard_candidate" >&2
      return 1
      ;;
  esac
}

resolve_prospective_output() (
  probe_parent=$1
  requested_name=$2
  missing_suffix=''
  while :; do
    if [ -L "$probe_parent" ] && [ ! -e "$probe_parent" ]; then
      printf 'refusing broken output-parent symlink: %s\n' "$probe_parent" >&2
      exit 1
    fi
    if [ -e "$probe_parent" ]; then
      if [ ! -d "$probe_parent" ]; then
        printf 'refusing non-directory output parent: %s\n' "$probe_parent" >&2
        exit 1
      fi
      canonical_parent=$(CDPATH= cd -- "$probe_parent" && pwd -P) || {
        printf 'cannot resolve output parent: %s\n' "$probe_parent" >&2
        exit 1
      }
      break
    fi
    missing_component=$(basename -- "$probe_parent")
    case "$missing_component" in
      ''|.|..)
        printf 'refusing invalid output-parent component: %s\n' "$probe_parent" >&2
        exit 1
        ;;
    esac
    missing_suffix="/$missing_component$missing_suffix"
    next_parent=$(dirname -- "$probe_parent")
    if [ "$next_parent" = "$probe_parent" ]; then
      printf 'cannot resolve output parent: %s\n' "$probe_parent" >&2
      exit 1
    fi
    probe_parent=$next_parent
  done
  if [ "$canonical_parent" = / ]; then
    if [ -n "$missing_suffix" ]; then
      printf '/%s/%s\n' "${missing_suffix#/}" "$requested_name"
    else
      printf '/%s\n' "$requested_name"
    fi
  else
    printf '%s%s/%s\n' "$canonical_parent" "$missing_suffix" "$requested_name"
  fi
)

guard_output_namespace "$lexical_output"
output_parent=$(dirname -- "$lexical_output")
output_name=$(basename -- "$lexical_output")
[ "$output_name" != '.' ] && [ "$output_name" != '..' ] || {
  printf 'refusing unsafe output path: %s\n' "$requested_output" >&2
  exit 1
}
if ! prospective_output=$(resolve_prospective_output "$output_parent" "$output_name"); then
  exit 1
fi
guard_output_namespace "$prospective_output"
output=$prospective_output
marker="$output.owned"
marker_value="akibiyori:demo-site:$mode:v1"
marker_matches() {
  [ -f "$1" ] && [ ! -L "$1" ] && [ "$(wc -l < "$1")" -eq 1 ] && [ "$(cat "$1")" = "$2" ]
}

if [ -L "$output" ]; then
  printf 'refusing symlink output: %s\n' "$output" >&2
  exit 1
elif [ -e "$output" ]; then
  if [ ! -d "$output" ] || ! marker_matches "$marker" "$marker_value"; then
    printf 'refusing to remove unowned output: %s\n' "$output" >&2
    exit 1
  fi
elif [ -e "$marker" ] || [ -L "$marker" ]; then
  if ! marker_matches "$marker" "$marker_value"; then
    printf 'refusing to remove unowned marker: %s\n' "$marker" >&2
    exit 1
  fi
fi

: "${AKIBIYORI:?set AKIBIYORI to the real built Akibiyori executable}"
: "${COBALT:=cobalt}"
: "${PAGEFIND:=pagefind}"
: "${SHOWCASE_MANIFEST:=$repo_root/akibiyori-showcase.json}"
[ -x "$AKIBIYORI" ] || {
  printf 'Akibiyori executable is not executable: %s\n' "$AKIBIYORI" >&2
  exit 1
}

site_ids='docs observatory planets kubernetes petstore'
expected_sites=$(printf '%s\n' $site_ids | paste -sd, -)

site_config_and_mount() {
  case "$1" in
    docs) printf '%s\n' 'akibiyori-docs.json' '' ;;
    observatory) printf '%s\n' 'examples/observatory/config.json' '/demos/observatory' ;;
    planets) printf '%s\n' 'examples/planets.json' '/demos/planets' ;;
    kubernetes) printf '%s\n' 'examples/kubernetes/config.json' '/demos/kubernetes' ;;
    petstore) printf '%s\n' 'examples/petstore/config.json' '/demos/petstore' ;;
    *) printf 'unsupported fixed showcase site: %s\n' "$1" >&2; return 1 ;;
  esac
}

preflight_root=$(mktemp -d "${TMPDIR:-/tmp}/akibiyori-showcase-preflight.XXXXXX")
trap 'rm -rf "$preflight_root"' EXIT HUP INT TERM

preflight_site() {
  site_id=$1
  site_config_and_mount "$site_id" > "$preflight_root/owner"
  config=$(sed -n '1p' "$preflight_root/owner")
  mount=$(sed -n '2p' "$preflight_root/owner")
  full_mount="$base_path$mount"
  "$AKIBIYORI" generate --config "$repo_root/$config" --output "$preflight_root/$site_id/source" --base-path "$full_mount" --showcase-manifest "$SHOWCASE_MANIFEST" --showcase-site "$site_id" --showcase-expected-sites "$expected_sites"
}

for site_id in $site_ids; do
  preflight_site "$site_id"
done

mkdir -p "$output_parent"
output_parent=$(CDPATH= cd -- "$output_parent" && pwd -P) || {
  printf 'cannot resolve created output parent: %s\n' "$requested_output" >&2
  exit 1
}
output="$output_parent/$output_name"
guard_output_namespace "$output"
marker="$output.owned"
if [ -L "$output" ]; then
  printf 'refusing symlink output: %s\n' "$output" >&2
  exit 1
elif [ -e "$output" ]; then
  if [ ! -d "$output" ] || ! marker_matches "$marker" "$marker_value"; then
    printf 'refusing to remove unowned output: %s\n' "$output" >&2
    exit 1
  fi
elif [ -e "$marker" ] || [ -L "$marker" ]; then
  if ! marker_matches "$marker" "$marker_value"; then
    printf 'refusing to remove unowned marker: %s\n' "$marker" >&2
    exit 1
  fi
fi

if [ -e "$output" ]; then
  rm -rf "$output"
fi
if [ -e "$marker" ] || [ -L "$marker" ]; then
  rm -f "$marker"
fi
mkdir -p "$output/builds" "$output/site"

build_site() {
  config=$1
  stage=$2
  mount=$3
  site_id=$4
  full_mount="$base_path$mount"
  "$AKIBIYORI" build --config "$repo_root/$config" --output "$stage" --base-path "$full_mount" --cobalt "$COBALT" --pagefind "$PAGEFIND" --showcase-manifest "$SHOWCASE_MANIFEST" --showcase-site "$site_id" --showcase-expected-sites "$expected_sites"
}

copy_site() {
  source_site=$1
  destination=$2
  [ -d "$source_site" ] && [ ! -L "$source_site" ] || {
    printf 'native site output is missing or unsafe: %s\n' "$source_site" >&2
    return 1
  }
  if find "$source_site" -type l -print -quit | grep -q .; then
    printf 'native site contains a symlink: %s\n' "$source_site" >&2
    return 1
  fi
  if [ -L "$destination" ]; then
    printf 'refusing symlink destination: %s\n' "$destination" >&2
    return 1
  fi
  find "$source_site" -type d -print | while IFS= read -r source_dir; do
    relative=${source_dir#"$source_site"}
    target="$destination$relative"
    if [ -e "$target" ] || [ -L "$target" ]; then
      if [ -L "$target" ] || [ ! -d "$target" ]; then
        printf 'site namespace collision at %s\n' "$target" >&2
        exit 1
      fi
    fi
  done
  find "$source_site" -type f -print | while IFS= read -r source_file; do
    relative=${source_file#"$source_site"/}
    target="$destination/$relative"
    if [ -e "$target" ] || [ -L "$target" ]; then
      printf 'site namespace collision at %s\n' "$target" >&2
      exit 1
    fi
  done
  mkdir -p "$destination"
  cp -R "$source_site"/. "$destination"/
}

for site_id in $site_ids; do
  site_config_and_mount "$site_id" > "$preflight_root/owner"
  config=$(sed -n '1p' "$preflight_root/owner")
  mount=$(sed -n '2p' "$preflight_root/owner")
  build_site "$config" "$output/builds/$site_id" "$mount" "$site_id"
  copy_site "$output/builds/$site_id/site" "$output/site"
done

printf '%s\n' "$marker_value" > "$marker"
printf 'assembled native site: %s\n' "$output/site"
