# Releasing health-cli

Standing rule (hx-rx3.25, cos/David 2026-10-13): **no `cargo build` or
`cargo test` on carbon.** The build boxes are Linux, so they cannot produce the
macOS binary carbon needs. GitHub Actions builds it instead, and carbon only
downloads and runs it.

- Repo: `github.com/davidawad/health-cli` (private mirror; GitLab stays canonical).
- Workflow: [`.github/workflows/release.yml`](../.github/workflows/release.yml),
  modelled on biomarker-cli's.
- Tap: `davidawad/homebrew-tap` (`brew tap davidawad/tap`).

## What a tag produces

Pushing a `v*` tag runs `release.yml`. It builds every binary in the package
(`health`, `supplement-index`) for two targets, each on a native macOS runner:

| target | runner | toolchain |
|---|---|---|
| `aarch64-apple-darwin` | `macos-latest` | stable |
| `x86_64-apple-darwin` | `macos-15-intel` | nightly (fsqlite 0.4 on x86_64) |

For each target it first checks that the tag equals `v` + the `Cargo.toml`
version. After the build it runs `health --version`, `health db init` and
`health doctor` against a throwaway database with an env key. It then attaches
these files to the GitHub release:

```
health-cli-<tag>-<target>.tar.gz          # health-cli-<tag>-<target>/{health,supplement-index,README.md,LICENSE}
health-cli-<tag>-<target>.tar.gz.sha256
SHA256SUMS                                # every archive
```

`workflow_dispatch` with a `tag` input rebuilds an existing tag and replaces
that release's assets (`--clobber`).

Only macOS is built because the repo is private, so macOS runner minutes are
billed. Linux and Windows builds are not needed because Linux boxes build from
source. To add a target, copy its matrix entry from biomarker-cli's
`release.yml`. `ci-full.yml` also runs on `v*` tags and runs the full
five-platform test matrix, so each tag currently pays for that run as well.

## Cutting a release (PM)

1. Bump `version` in `Cargo.toml` (and `Cargo.lock`, using `cargo update -p health-cli --offline`
   **on a box**). Land the bump through the normal queue.
2. Tag the landed commit and push only to the GitHub mirror:
   `git tag v0.1.0 <sha> && git push github v0.1.0`.
3. Wait for the `Release` run (`gh run watch --repo davidawad/health-cli`).
   Then check `gh release view v0.1.0 --repo davidawad/health-cli`.
4. Update the tap formula (below) with the new version and the two `sha256`
   values from `SHA256SUMS`.

Agents do not create or push tags. The PM does.

## Homebrew formula (`davidawad/homebrew-tap`, `Formula/health-cli.rb`)

The other formulas follow the same pattern as `Formula/biomarker-cli.rb`:
prebuilt release tarballs selected per platform with
`on_macos`/`on_arm`/`on_intel`, and `install` puts the binary in place and
generates completions and man pages from it. There is one difference.
**biomarker-cli and genome-cli are public, so their formulas use plain
`releases/download/...` URLs. health-cli is private, so the same URL returns
404 to anonymous `curl`.** The formula therefore needs a download strategy
that fetches the release asset through the GitHub API with a token. Homebrew
removed `GitHubPrivateRepositoryReleaseDownloadStrategy` from core, so the
formula defines it in the tap. It reads `HOMEBREW_GITHUB_API_TOKEN`, which
must be a token that can read `davidawad/health-cli`
(`export HOMEBREW_GITHUB_API_TOKEN=$(gh auth token)` on carbon works).

```ruby
require "download_strategy"

# Release assets of a private repo: resolve the asset id via the API, then
# download it with Accept: application/octet-stream and the user's token.
class GitHubPrivateRepositoryReleaseDownloadStrategy < CurlDownloadStrategy
  def initialize(url, name, version, **meta)
    super
    match = %r{^https://github\.com/([^/]+)/([^/]+)/releases/download/([^/]+)/(\S+)$}.match(@url)
    raise CurlDownloadStrategyError, "Invalid GitHub release URL: #{@url}" unless match

    _, @owner, @repo, @tag, @filename = *match
  end

  private

  def token
    Homebrew::EnvConfig.github_api_token.presence ||
      raise(CurlDownloadStrategyError, "Set HOMEBREW_GITHUB_API_TOKEN to a token that can read #{@owner}/#{@repo}")
  end

  def asset_url
    release = GitHub::API.open_rest("https://api.github.com/repos/#{@owner}/#{@repo}/releases/tags/#{@tag}")
    asset = release["assets"].find { |a| a["name"] == @filename }
    raise CurlDownloadStrategyError, "Asset #{@filename} not found in #{@tag}" unless asset

    asset["url"]
  end

  def _fetch(url:, resolved_url:, timeout:)
    curl_download asset_url,
                  "--header", "Accept: application/octet-stream",
                  "--header", "Authorization: Bearer #{token}",
                  to: temporary_path, timeout:
  end
end

class HealthCli < Formula
  desc "Non-lab personal health data (supplements, nutrition, routines...), encrypted at rest"
  homepage "https://github.com/davidawad/health-cli"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/davidawad/health-cli/releases/download/v0.1.0/health-cli-v0.1.0-aarch64-apple-darwin.tar.gz",
          using: GitHubPrivateRepositoryReleaseDownloadStrategy
      sha256 "<sha256 from SHA256SUMS>"
    end
    on_intel do
      url "https://github.com/davidawad/health-cli/releases/download/v0.1.0/health-cli-v0.1.0-x86_64-apple-darwin.tar.gz",
          using: GitHubPrivateRepositoryReleaseDownloadStrategy
      sha256 "<sha256 from SHA256SUMS>"
    end
  end

  # Prebuilt release binaries only: fsqlite 0.4 needs nightly Rust on x86_64,
  # which Homebrew does not ship, and carbon must not compile (hx-rx3.25).
  def install
    bin.install "health", "supplement-index"
    generate_completions_from_executable(bin/"health", "completions")
    system bin/"health", "man", "--dir", man1
  end

  def caveats
    <<~EOS
      Private release: installs and upgrades need HOMEBREW_GITHUB_API_TOKEN
      with read access to davidawad/health-cli (e.g. $(gh auth token)).
      The database is encrypted with your SSH key (~/.ssh/id_ed25519) by default; see `health doctor`.
    EOS
  end

  test do
    ENV["HEALTH_DB"] = testpath/"test.db"
    ENV["HEALTH_KEY_SOURCE"] = "env"
    ENV["HEALTH_KEY"] = "brew-test-passphrase"
    system bin/"health", "db", "init"
    system bin/"health", "doctor"
    refute_match "SQLite format", (testpath/"test.db").binread
  end
end
```

Notes:

- Homebrew takes the version from the `releases/download/vX.Y.Z/` URLs. Bump
  both URLs and both `sha256` values together.
- A public tap that names a private repo exposes only the repo name and version
  numbers. The assets stay behind the token.
- The `GitHubPrivateRepositoryReleaseDownloadStrategy` above has not yet been
  run against a real release. Verify it on carbon with
  `brew install --verbose --debug davidawad/tap/health-cli` the first time.
- Add a `health-cli` row to the tap README's formula table
  (Platforms: "macOS arm64/x86_64 (prebuilt, private: needs `HOMEBREW_GITHUB_API_TOKEN`)").

## Without Homebrew

You can also install on carbon from a release without the tap. This needs no
compiler:

```console
$ tag=v0.1.0 t=aarch64-apple-darwin
$ cd "$(mktemp -d)"
$ gh release download "$tag" --repo davidawad/health-cli -p "health-cli-$tag-$t.tar.gz*"
$ shasum -a 256 -c "health-cli-$tag-$t.tar.gz.sha256"
$ tar -xzf "health-cli-$tag-$t.tar.gz"
$ install -m 0755 "health-cli-$tag-$t/health" ~/.local/bin/health
$ health doctor
```
