class Pin < Formula
  desc "Terminal sticky-note HUD for human-agent coding sessions"
  homepage "https://github.com/abhidrona/pin"
  url "https://github.com/abhidrona/pin.git", tag: "v0.1.0"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
    man1.install "man/man1/pin.1"
  end

  test do
    system bin/"pin", "--version"

    (testpath/"repo").mkpath
    cd testpath/"repo" do
      system bin/"pin", "add", "brew smoke test"
      assert_path_exists testpath/"repo/.pin/log.jsonl"
      assert_match "brew smoke test", shell_output("#{bin}/pin ls")
    end
  end
end
