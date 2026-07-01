class CcProfile < Formula
  desc "Profile management for Claude Code endpoints and models"
  homepage "https://github.com/therealhieu/cc-profile"
  url "https://github.com/therealhieu/cc-profile/archive/refs/tags/v0.1.0.tar.gz"
  # Update sha256 when cutting a new release tag (brew fetch --force --build-from-source ./Formula/cc-profile.rb).
  sha256 "eb4dc3926f024a6a5379151e98f5b722c4f9853806b4e3b46d5208ce1fd25887"
  license "MIT"
  head "https://github.com/therealhieu/cc-profile.git", branch: "master"

  livecheck do
    url :stable
    strategy :github_latest
  end

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--path", ".", "--root", prefix
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/cc-profile --version")
  end
end