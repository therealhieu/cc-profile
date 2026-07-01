class CcProfile < Formula
  desc "Profile management for Claude Code endpoints and models"
  homepage "https://github.com/therealhieu/cc-profile"
  url "https://github.com/therealhieu/cc-profile/archive/refs/tags/v0.1.0.tar.gz"
  # Update sha256 when cutting a new release tag (brew fetch --force --build-from-source ./Formula/cc-profile.rb).
  sha256 "bf15dc3bd1f2b5d7e27d339626f47715a3416e44383afca0c15144586a6e7731"
  license "MIT"
  head "https://github.com/therealhieu/cc-profile.git", branch: "master"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", ".", "--root", prefix
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/cc-profile --version")
  end

  livecheck do
    url :stable
    strategy :github_latest
  end
end