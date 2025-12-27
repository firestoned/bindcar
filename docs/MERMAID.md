# Mermaid Diagrams in bindcar Documentation

This document explains how to use Mermaid diagrams in the bindcar documentation.

## Overview

The bindcar documentation uses [Mermaid](https://mermaid.js.org/) for creating diagrams directly in Markdown files. The diagrams are rendered client-side in the browser using the Mermaid JavaScript library.

## Configuration

### Files Involved

1. **`mermaid.min.js`** - The Mermaid library (2.5MB)
2. **`mermaid-init.js`** - Custom initialization and configuration
3. **`theme/custom.css`** - Styles for diagram rendering and pan/zoom
4. **`book.toml`** - mdBook configuration with Mermaid preprocessor

### Key Features

- ✅ **Pan and Zoom** - Diagrams support scrolling and zooming
- ✅ **Theme Integration** - Diagrams adapt to light/dark themes
- ✅ **No DOM Conflicts** - Proper initialization prevents mdBook conflicts
- ✅ **Responsive Design** - Works on mobile and desktop
- ✅ **Git-Friendly** - All Mermaid files are committed to the repository

## Usage

### Basic Diagram

To add a Mermaid diagram, use a fenced code block with `mermaid` as the language:

\`\`\`mermaid
graph LR
    A[Client] -->|HTTP Request| B[bindcar API]
    B -->|RNDC Command| C[BIND9]
    C -->|Response| B
    B -->|JSON Response| A
\`\`\`

### Supported Diagram Types

Mermaid supports many diagram types:

#### 1. Flowcharts
\`\`\`mermaid
graph TD
    A[Start] --> B{Is it?}
    B -->|Yes| C[OK]
    C --> D[Rethink]
    D --> B
    B ---->|No| E[End]
\`\`\`

#### 2. Sequence Diagrams
\`\`\`mermaid
sequenceDiagram
    participant Client
    participant API as bindcar API
    participant BIND9

    Client->>API: POST /zones
    API->>BIND9: rndc addzone
    BIND9-->>API: Success
    API-->>Client: 201 Created
\`\`\`

#### 3. Class Diagrams
\`\`\`mermaid
classDiagram
    class ZoneConfig {
        +String zoneName
        +String zoneType
        +SoaRecord soa
        +create()
        +delete()
    }
    class SoaRecord {
        +String primaryNs
        +String adminEmail
        +int serial
    }
    ZoneConfig --> SoaRecord
\`\`\`

#### 4. State Diagrams
\`\`\`mermaid
stateDiagram-v2
    [*] --> Created
    Created --> Active: rndc addzone
    Active --> Frozen: rndc freeze
    Frozen --> Active: rndc thaw
    Active --> Deleted: rndc delzone
    Deleted --> [*]
\`\`\`

#### 5. Entity Relationship Diagrams
\`\`\`mermaid
erDiagram
    ZONE ||--o{ DNS_RECORD : contains
    ZONE {
        string name
        string type
        int serial
    }
    DNS_RECORD {
        string name
        string type
        string value
        int ttl
    }
\`\`\`

## Pan and Zoom

### Desktop
- **Scroll**: Use mouse wheel or trackpad to scroll through large diagrams
- **Pan**: Click and drag to move the diagram
- **Cursor**: Changes to "grab" when hovering over diagrams

### Mobile
- **Pinch to Zoom**: Use two fingers to zoom in/out
- **Pan**: Swipe to move the diagram
- **Scroll**: Swipe up/down to scroll

## Troubleshooting

### Diagram Not Rendering

1. **Check Syntax**: Ensure your Mermaid syntax is valid
   - Test it on [Mermaid Live Editor](https://mermaid.live/)
2. **Clear Browser Cache**: Hard refresh (Cmd+Shift+R / Ctrl+Shift+F5)
3. **Check Console**: Open browser dev tools and look for errors

### DOM Conflicts

If you see duplicate diagrams or rendering issues:

1. The `mermaid-init.js` includes a `DOMContentLoaded` listener that waits for mdBook to finish
2. The `suppressErrorRendering: true` option prevents duplicate error messages
3. CSS ensures `[data-processed="true"]` elements are properly displayed

### Theme Not Updating

When switching between light and dark themes:

1. The page automatically reloads to re-render diagrams
2. This is intentional to ensure proper theme application
3. Diagrams will use `default` theme for light mode and `dark` theme for dark mode

## Best Practices

### 1. Keep Diagrams Simple
- Focus on clarity over complexity
- Use subgraphs for organization
- Add comments with `%%` for documentation

### 2. Use Consistent Styling
- Use the same diagram type for similar concepts
- Follow existing diagram patterns in the docs
- Use meaningful node IDs and labels

### 3. Optimize for Readability
- Use clear, descriptive labels
- Avoid crossing lines when possible
- Group related elements together

### 4. Test on Multiple Devices
- Check diagrams on desktop and mobile
- Verify pan/zoom works correctly
- Ensure text is readable at all sizes

## Advanced Configuration

### Mermaid Initialization Options

The `mermaid-init.js` file includes these key configurations:

```javascript
mermaid.initialize({
    startOnLoad: true,           // Auto-render on page load
    theme: 'default',            // Theme based on mdBook theme
    securityLevel: 'loose',      // Enable pan/zoom
    suppressErrorRendering: true, // Prevent duplicate errors
    flowchart: {
        useMaxWidth: true,       // Responsive width
        htmlLabels: true,        // Rich text labels
        curve: 'basis'           // Smooth curves
    }
});
```

### Custom Styling

The `theme/custom.css` file includes:

- `.mermaid-container` - Wrapper with borders and scrolling
- `pre.mermaid svg` - Grab cursor and responsive sizing
- Dark theme adjustments for all themes (coal, navy, ayu)
- Mobile-responsive max-heights and overflow

## Files to Commit

**Always commit these files:**
- ✅ `mermaid.min.js` - Required for rendering
- ✅ `mermaid-init.js` - Required for initialization
- ✅ `theme/custom.css` - Required for styling
- ✅ All Markdown files with `` ```mermaid `` blocks

**Never commit:**
- ❌ `target/` - Build output
- ❌ `.mermaid-*` - Temporary files (if generated)
- ❌ `mermaid-diagram-*.svg` - Auto-generated SVGs

## Resources

- [Mermaid Documentation](https://mermaid.js.org/)
- [Mermaid Live Editor](https://mermaid.live/) - Test diagrams online
- [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid) - The preprocessor we use
- [Mermaid Syntax Reference](https://mermaid.js.org/intro/syntax-reference.html)

## Examples in bindcar Docs

Check these files for examples:
- `docs/src/concepts/architecture.md` - High-level architecture diagrams
- `docs/src/guide/creating-zones.md` - Zone creation flow
- `docs/src/developer-guide/rndc-integration.md` - RNDC integration sequence

## Support

If you encounter issues with Mermaid diagrams:
1. Check this document first
2. Test your diagram on [Mermaid Live Editor](https://mermaid.live/)
3. Review browser console for errors
4. Open an issue on the bindcar GitHub repository
