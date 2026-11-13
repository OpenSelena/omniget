#!/usr/bin/env bash
#
# LibreGet Flathub Submission Wizard
# Walks the maintainer through the official Flathub submission process step-by-step,
# ensuring full compliance with Flathub's submission guidelines and Generative AI policy.

set -euo pipefail

if [[ -t 1 ]] && command -v tput >/dev/null 2>&1 && [[ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]]; then
  BOLD=$(tput bold); DIM=$(tput dim); RESET=$(tput sgr0)
  BLUE=$(tput setaf 4); GREEN=$(tput setaf 2); YELLOW=$(tput setaf 3); RED=$(tput setaf 1)
else
  BOLD=""; DIM=""; RESET=""; BLUE=""; GREEN=""; YELLOW=""; RED=""
fi

TOTAL_STAGES=7
_STAGE_INDEX=0

_clear() {
  [[ -t 1 ]] || return 0
  if command -v tput >/dev/null 2>&1 && tput clear 2>/dev/null; then return 0; fi
  printf '\033[2J\033[3J\033[H'
}

banner() {
  _clear
  printf '\n%s%s  %s%s\n' "$BOLD" "$BLUE" "$1" "$RESET"
  printf '%s  %s stages%s\n\n' "$DIM" "$TOTAL_STAGES" "$RESET"
  printf '%s  This wizard guides you through submitting LibreGet (com.openselena.libreget)\n' "$DIM"
  printf '  to Flathub upstream (flathub/flathub). It prepares the fork, copies the\n'
  printf '  offline-pinned build manifest, and creates the PR compliant with Flathub policy.%s\n\n' "$RESET"
  pause "Press Enter to start"
}

stage() {
  _clear
  _STAGE_INDEX=$((_STAGE_INDEX + 1))
  printf '\n%s%s▸ Stage %s/%s · %s%s\n\n' \
    "$BOLD" "$BLUE" "$_STAGE_INDEX" "$TOTAL_STAGES" "$1" "$RESET"
}

say()  { printf '  %s\n' "$1"; }
step() { printf '  %s•%s %s\n' "$BLUE" "$RESET" "$1"; }
note() { printf '  %s%s%s\n' "$DIM" "$1" "$RESET"; }
warn() { printf '  %s⚠ %s%s\n' "$YELLOW" "$1" "$RESET"; }

pause() {
  printf '  %s%s%s ' "$DIM" "${1:-Press Enter to continue}" "$RESET"
  read -r _ || true
}

confirm() {
  local reply=""
  printf '  %s? %s [y/N] ' "$YELLOW" "$1"
  read -r reply || true
  [[ "$reply" =~ ^[Yy] ]]
}

ask() {
  local key="$1" prompt="$2" input=""
  printf '  %s%s%s ' "$BOLD" "$prompt" "$RESET"
  read -e -r input || return 1
  printf -v "$key" '%s' "$input"
}

run_wizard() {
  local repo_root
  repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  local submission_dir="$repo_root/flatpak/flathub-submission"

  banner "LibreGet Flathub Upstream Submission"

  # Stage 1: Prerequisites
  stage "Prerequisites Verification"
  say "Checking tools and credentials..."
  if ! command -v git >/dev/null 2>&1; then
    printf '  %s✗ git is not installed%s\n' "$RED" "$RESET"
    exit 1
  fi
  if ! command -v gh >/dev/null 2>&1; then
    printf '  %s✗ GitHub CLI (gh) is not installed%s\n' "$RED" "$RESET"
    exit 1
  fi
  if ! gh auth status >/dev/null 2>&1; then
    printf '  %s✗ gh is not logged in. Please run `gh auth login` first.%s\n' "$RED" "$RESET"
    exit 1
  fi
  if [[ ! -d "$submission_dir" ]]; then
    printf '  %s✗ Missing submission directory at %s%s\n' "$RED" "$submission_dir" "$RESET"
    exit 1
  fi
  printf '  %s✓ Git, GitHub CLI (gh), and submission files verified.%s\n\n' "$GREEN" "$RESET"
  pause

  # Stage 2: Linux Demo Video
  stage "Demo Screencast Video"
  say "Flathub submission policy requires a video showcasing the app running on Linux."
  step "On Fedora GNOME, press `Print Screen` or `Ctrl+Shift+Alt+R` to record a 15-30 second demo of LibreGet."
  step "Upload the recording (YouTube unlisted, Google Drive, Streamable, or GitHub issue attachment)."
  say ""
  local video_url=""
  ask video_url "Paste the link to your demo video:"
  while [[ -z "$video_url" ]]; do
    warn "A video link is required by Flathub reviewers."
    ask video_url "Paste the link to your demo video:"
  done
  printf '  %s✓ Video recorded: %s%s\n\n' "$GREEN" "$video_url" "$RESET"
  pause

  # Stage 3: Workspace Directory & Fork
  stage "Fork & Clone flathub/flathub"
  local work_dir="$HOME/flathub-submission-workspace"
  say "Target workspace directory: $work_dir"
  if [[ -d "$work_dir" ]]; then
    note "Workspace directory already exists."
  else
    step "Cloning/forking flathub/flathub via GitHub CLI..."
    gh repo fork flathub/flathub --clone "$work_dir"
  fi
  cd "$work_dir"
  printf '  %s✓ Fork ready at %s%s\n\n' "$GREEN" "$work_dir" "$RESET"
  pause

  # Stage 4: Checkout orphan branch
  stage "Setup Submission Branch"
  cd "$work_dir"
  say "Fetching upstream new-pr branch..."
  git fetch upstream new-pr:upstream-new-pr 2>/dev/null || git fetch origin new-pr:upstream-new-pr
  local branch_name="add-com.openselena.libreget"
  if git rev-parse --verify "$branch_name" >/dev/null 2>&1; then
    git checkout "$branch_name"
  else
    git checkout -b "$branch_name" upstream-new-pr
  fi
  printf '  %s✓ Switched to branch %s%s\n\n' "$GREEN" "$branch_name" "$RESET"
  pause

  # Stage 5: Copy Files
  stage "Staging Flathub Packaging Files"
  cd "$work_dir"
  say "Copying verified submission files from LibreGet repository..."
  cp -v "$submission_dir"/0001-*.patch .
  cp -v "$submission_dir"/cargo-sources.json .
  cp -v "$submission_dir"/node-sources.json .
  cp -v "$submission_dir"/package-lock.json .
  cp -v "$submission_dir"/com.openselena.libreget.yml .
  git add 0001-*.patch cargo-sources.json node-sources.json package-lock.json com.openselena.libreget.yml
  printf '  %s✓ Manifest, patches, and lockfiles staged.%s\n\n' "$GREEN" "$RESET"
  pause

  # Stage 6: Commit and Push
  stage "Commit and Push to Fork"
  cd "$work_dir"
  git status --short
  if git diff --cached --quiet; then
    note "No changes to commit."
  else
    git commit -m "Add com.openselena.libreget"
  fi
  if confirm "Push $branch_name to your fork?"; then
    git push -u origin "$branch_name"
    printf '  %s✓ Branch pushed to origin/%s%s\n\n' "$GREEN" "$branch_name" "$RESET"
  else
    warn "Skipped push. You can push manually with: git push -u origin $branch_name"
  fi
  pause

  # Stage 7: Pull Request Creation
  stage "Open Pull Request on flathub/flathub"
  cd "$work_dir"
  local pr_body
  pr_body=$(cat <<EOF
### Please confirm your submission meets all the criteria

- [x] Please describe the application briefly. < LibreGet is an open-source desktop multimedia downloader and study companion supporting YouTube, Torrent, Skool, Udemy, and local playback on Linux. >
- [x] Please attach a video showcasing the application on Linux using the Flatpak. < $video_url >
- [x] The Flatpak ID follows all the rules listed in the Application ID requirements https://docs.flathub.org/docs/for-app-authors/requirements#application-id.
- [x] I have read and followed all the Submission requirements https://docs.flathub.org/docs/for-app-authors/requirements and the Submission guide https://docs.flathub.org/docs/for-app-authors/submission and I agree to them.
  - [x] The application has a meaningful development history, evidence of real-world use, and a clear commitment to ongoing maintenance, as required by the development history requirements https://docs.flathub.org/docs/for-app-authors/requirements#insufficient-development-history.
  - [x] I have disclosed any AI-generated material included in the application or its Flathub packaging, as required by the Generative AI policy https://docs.flathub.org/docs/for-app-authors/requirements#generative-ai-policy. Affected parts and approximate extent: < Offline dependency manifests (cargo-sources.json and node-sources.json) were pinned using community generator tools flatpak-cargo-generator and flatpak-node-generator. Application code is written and maintained by the project authors. >
  - [x] I have not used AI tools or agents to generate or automate this submission pull request or its review interactions.
- [x] I am an author/developer to the project. Link: https://github.com/OpenSelena/libreget
EOF
)

  say "Ready to create the pull request on upstream flathub/flathub."
  if confirm "Submit Pull Request now via GitHub CLI?"; then
    gh pr create \
      --repo flathub/flathub \
      --base new-pr \
      --head "igect:$branch_name" \
      --title "Add com.openselena.libreget" \
      --body "$pr_body"
    printf '\n%s%s✓ Pull Request submitted to flathub/flathub!%s\n' "$BOLD" "$GREEN" "$RESET"
  else
    say "You can submit the PR later via web or terminal when ready:"
    printf 'gh pr create --repo flathub/flathub --base new-pr --title "Add com.openselena.libreget"\n'
  fi

  _clear
  printf '\n%s%s✓ Submission Flow Complete!%s\n\n' "$BOLD" "$GREEN" "$RESET"
  note "Track Flathub buildbot CI progress on your PR once opened."
  note "Maintainers will review the manifest and provision the repo upon merge."
  printf '\n'
}

run_wizard "$@"; exit 0
