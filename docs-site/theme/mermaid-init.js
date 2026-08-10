(async () => {
  const blocks = Array.from(document.querySelectorAll("pre code.language-mermaid"));
  if (blocks.length === 0) {
    return;
  }

  const { default: mermaid } = await import("https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs");
  mermaid.initialize({ startOnLoad: false, theme: "neutral" });

  for (const block of blocks) {
    const container = document.createElement("div");
    container.className = "mermaid";
    container.textContent = block.textContent;
    block.closest("pre").replaceWith(container);
  }

  await mermaid.run({ querySelector: ".mermaid" });
})();
