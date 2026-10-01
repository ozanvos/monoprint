const examples = {
  cli: ['cargo build --locked --release\n\n./target/release/monoprint main.rs \\\n  --theme gruvbox-medium --padding 32 \\\n  --scale 2 -o code.png', 'Build from source with Rust 1.97+ and a C compiler. Prebuilt downloads are in preparation.'],
  rust: ['use monoprint::{Renderer, Scale, Snippet, Style};\n\nlet image = Renderer::new().png(\n    &Snippet::new("fn main() {}").language("rust".parse()?),\n    &Style::default(),\n    Scale::X2,\n)?;', 'Use a source/path dependency with default-features = false. crates.io publication is not available yet.'],
  typescript: ['import { writeFile } from "node:fs/promises";\nimport { png } from "monoprint";\n\nconst image = await png("const answer = 42;", {\n  language: "ts", theme: "gruvbox-medium", scale: 2,\n});\nawait writeFile("code.png", image);', 'Node.js 22+. npm packages are in preparation; build and install local tarballs from the source repository.'],
};
document.querySelectorAll('[data-theme]').forEach(button => button.addEventListener('click', () => {
  document.querySelectorAll('[data-theme]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
  const sample = document.getElementById('sample');
  sample.src = `assets/${button.dataset.theme}.png`;
  sample.alt = `A Rust snippet rendered with ${button.textContent}`;
  document.getElementById('sample-name').textContent = button.textContent;
}));
document.querySelectorAll('[data-example]').forEach(button => button.addEventListener('click', () => {
  document.querySelectorAll('[data-example]').forEach(item => item.setAttribute('aria-pressed', String(item === button)));
  const [code, note] = examples[button.dataset.example];
  document.getElementById('example').textContent = code;
  document.getElementById('interface-note').textContent = note;
}));
