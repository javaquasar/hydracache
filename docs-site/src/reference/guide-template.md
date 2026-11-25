# Guide Template

Every guide under *Local Cache*, *Database*, *Cluster and Server*, and *Operations* follows the same shape, so readers know where to look and reviewers can spot what is missing.

1. **Title** — the task, not the type name.
2. **API line** — `**API:**` followed by docs.rs links to the main types, macros, or crates.
3. **At a glance** — a `[!NOTE]` block with five bullets:
   - **Use when** — the situation this guide is for;
   - **Crate** — what to add to `Cargo.toml`;
   - **Guarantees** — what the reader can rely on;
   - **Watch** — the counters, endpoints, or reports that show it working;
   - **Pitfalls** — the mistakes that make it wrong.
4. **Body** — explanation and examples. Runnable examples live in `docs-site/examples` and are pulled into the page with mdBook include directives; see the [Examples Contract](examples-contract.md).
5. **Caveats** — use `[!WARNING]`, `[!IMPORTANT]`, or `[!TIP]` blocks for limits the reader must not miss.
6. **Next Steps** — two or three links with a short reason each.

```markdown
# Do the Task

**API:** [`Type`](https://docs.rs/...)

> [!NOTE]
> **At a glance**
>
> - **Use when:** ...
> - **Crate:** ...
> - **Guarantees:** ...
> - **Watch:** ...
> - **Pitfalls:** ...

...

## Next Steps

- [Getting Started](../getting-started.md) — why to read it next
```
