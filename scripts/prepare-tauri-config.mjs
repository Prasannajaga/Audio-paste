import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const repoRoot = process.cwd();
const generatedConfigPath = path.join(
  repoRoot,
  "src-tauri",
  "tauri.generated.conf.json"
);

const normalizedOs = (() => {
  const explicit = process.env.PREPARE_TAURI_TARGET_OS;
  if (explicit) {
    return explicit;
  }

  switch (os.platform()) {
    case "win32":
      return "Windows";
    case "linux":
      return "Linux";
    case "darwin":
      return "Darwin";
    default:
      return os.platform();
  }
})();

const sourceRoot = path.join(repoRoot, "src-tauri");
const resourceExists = (relativePath) =>
  fs.existsSync(path.join(sourceRoot, relativePath));

const resolveFirstExisting = (paths) => {
  const match = paths.find(resourceExists);
  if (!match) {
    throw new Error(
      `Missing required resource. Checked: ${paths.join(", ")}`
    );
  }
  return match;
};

const generatedConfig = {
  bundle: {
    active: true,
  },
};

if (normalizedOs === "Linux") {
  generatedConfig.bundle.targets = ["deb"];
  generatedConfig.bundle.resources = [
    "whisper.cpp/build/bin/whisper-cli",
    resolveFirstExisting([
      "whisper.cpp/build/src/libwhisper.so.1",
      "whisper.cpp/build/src/libwhisper.so",
    ]),
    resolveFirstExisting([
      "whisper.cpp/build/ggml/src/libggml.so.0",
      "whisper.cpp/build/ggml/src/libggml.so",
    ]),
    resolveFirstExisting([
      "whisper.cpp/build/ggml/src/libggml-base.so.0",
      "whisper.cpp/build/ggml/src/libggml-base.so",
    ]),
    resolveFirstExisting([
      "whisper.cpp/build/ggml/src/libggml-cpu.so.0",
      "whisper.cpp/build/ggml/src/libggml-cpu.so",
    ]),
  ];
} else if (normalizedOs === "Windows") {
  generatedConfig.bundle.targets = ["nsis"];
  generatedConfig.bundle.resources = [
    "whisper.cpp/build/bin/whisper-cli.exe",
    "whisper.cpp/build/bin/whisper.dll",
    "whisper.cpp/build/bin/ggml.dll",
    "whisper.cpp/build/bin/ggml-base.dll",
    "whisper.cpp/build/bin/ggml-cpu.dll",
  ];

  for (const resource of generatedConfig.bundle.resources) {
    if (!resourceExists(resource)) {
      throw new Error(`Missing required Windows resource: ${resource}`);
    }
  }
} else {
  generatedConfig.bundle.targets = ["app"];
  generatedConfig.bundle.resources = ["whisper.cpp/build/bin/whisper-cli"];
}

fs.writeFileSync(generatedConfigPath, `${JSON.stringify(generatedConfig, null, 2)}\n`);
