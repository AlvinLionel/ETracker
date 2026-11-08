import { copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = join(root, "public", "ocr");
const coreOutput = join(output, "core");
const languageOutput = join(output, "lang");
const coreDirectory = join(root, "node_modules", "tesseract.js-core");
const languageDirectory = join(root, "node_modules", "@tesseract.js-data", "eng");

await Promise.all([mkdir(coreOutput, { recursive: true }), mkdir(languageOutput, { recursive: true })]);
await Promise.all([
  copyFile(join(root, "node_modules", "tesseract.js", "dist", "worker.min.js"), join(output, "worker.min.js")),
  copyFile(join(languageDirectory, "4.0.0_best_int", "eng.traineddata.gz"), join(languageOutput, "eng.traineddata.gz")),
  ...[
    "tesseract-core-lstm.wasm.js",
    "tesseract-core-simd-lstm.wasm.js",
    "tesseract-core-relaxedsimd-lstm.wasm.js",
  ].map((file) => copyFile(join(coreDirectory, file), join(coreOutput, file))),
]);

const [apacheLicense, modelPackage, regeneratorLicense, zlibLicense, workerLicenseNotices] = await Promise.all([
  readFile(join(coreDirectory, "LICENSE"), "utf8"),
  readFile(join(languageDirectory, "package.json"), "utf8").then(JSON.parse),
  readFile(join(root, "node_modules", "regenerator-runtime", "LICENSE"), "utf8"),
  readFile(join(root, "node_modules", "zlibjs", "LICENSE"), "utf8"),
  readFile(join(root, "node_modules", "tesseract.js", "dist", "worker.min.js.LICENSE.txt"), "utf8"),
]);
await writeFile(join(output, "THIRD_PARTY_NOTICES.txt"), [
  "OCR assets bundled with ETracker",
  "",
  "Tesseract.js and tesseract.js-core",
  "Licensed under the Apache License, Version 2.0. The full license follows.",
  "",
  apacheLicense.trim(),
  "",
  `English OCR model (@tesseract.js-data/eng ${modelPackage.version})`,
  `Copyright (c) ${modelPackage.author}`,
  "Licensed under the MIT License.",
  "",
  "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:",
  "",
  "The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.",
  "",
  "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.",
  "",
  "Worker bundle notices (buffer, ieee754, regenerator-runtime and zlib.js):",
  workerLicenseNotices.trim(),
  "",
  "buffer browser module — MIT License",
  "Copyright (c) Feross Aboukhadijeh",
  "Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the \"Software\"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:",
  "The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.",
  "THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.",
  "",
  "ieee754 — BSD 3-Clause License",
  "Copyright (c) Feross Aboukhadijeh. All rights reserved.",
  "Redistribution and use in source and binary forms, with or without modification, are permitted provided that the following conditions are met:",
  "1. Redistributions of source code must retain the above copyright notice, this list of conditions and the following disclaimer.",
  "2. Redistributions in binary form must reproduce the above copyright notice, this list of conditions and the following disclaimer in the documentation and/or other materials provided with the distribution.",
  "3. Neither the name of the copyright holder nor the names of its contributors may be used to endorse or promote products derived from this software without specific prior written permission.",
  "THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDER AND CONTRIBUTORS \"AS IS\" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.",
  "",
  "regenerator-runtime — MIT License",
  regeneratorLicense.trim(),
  "",
  "zlib.js — MIT License",
  zlibLicense.trim(),
  "",
].join("\n"));
