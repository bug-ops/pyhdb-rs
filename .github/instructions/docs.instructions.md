---
applyTo: "docs/**/*,*.md,!.local/**/*"
---

# Documentation Guidelines

## Content Quality

- **Accurate code examples** - verify examples compile/run
- **Up-to-date API references** - match current implementation
- **Clear installation instructions** - include platform-specific notes where needed
- All documentation must be in English

## Markdown Standards

- Valid markdown syntax - no broken links, proper formatting
- Consistent heading hierarchy: H1 > H2 > H3
- Code blocks with language tags: `python`, `rust`, `bash`, `yaml`, `toml`
- Use fenced code blocks with triple backticks

## API Documentation

- Document all public APIs with clear descriptions
- Include usage examples for complex functions
- Document return types and possible exceptions
- Note any side effects or thread-safety considerations

## Code Examples

```rust
// Use language-specific syntax highlighting
fn example() -> Result<(), Error> {
    Ok(())
}
```

```python
# Python examples should include type hints
def example() -> None:
    pass
```

## Project-Specific Documentation

- Architecture diagrams use ASCII art or Mermaid
- Type mapping tables for HANA to Arrow conversions
- Build commands in code blocks
- Environment variables in tables with descriptions
