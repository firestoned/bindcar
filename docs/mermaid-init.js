// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

(() => {
    const darkThemes = ['ayu', 'navy', 'coal'];
    const lightThemes = ['light', 'rust'];

    const classList = document.getElementsByTagName('html')[0].classList;

    let lastThemeWasLight = true;
    for (const cssClass of classList) {
        if (darkThemes.includes(cssClass)) {
            lastThemeWasLight = false;
            break;
        }
    }

    const theme = lastThemeWasLight ? 'default' : 'dark';

    // Initialize Mermaid with proper configuration for panning and zooming
    mermaid.initialize({
        startOnLoad: true,
        theme,
        // Enable pan and zoom functionality
        securityLevel: 'loose',
        // Prevent Mermaid from creating duplicate elements
        suppressErrorRendering: true,
        // Flowchart configuration for better rendering
        flowchart: {
            useMaxWidth: true,
            htmlLabels: true,
            curve: 'basis'
        },
        // Sequence diagram configuration
        sequence: {
            useMaxWidth: true,
            wrap: true
        },
        // Class diagram configuration
        class: {
            useMaxWidth: true
        },
        // State diagram configuration
        state: {
            useMaxWidth: true
        },
        // ER diagram configuration
        er: {
            useMaxWidth: true
        },
        // Journey diagram configuration
        journey: {
            useMaxWidth: true
        }
    });

    // Fix for DOM manipulation issues - ensure Mermaid doesn't interfere with mdBook
    document.addEventListener('DOMContentLoaded', () => {
        // Wait for mdBook to finish rendering before processing Mermaid diagrams
        setTimeout(() => {
            // Re-process any diagrams that weren't rendered on initial load
            const unrenderedDiagrams = document.querySelectorAll('.language-mermaid:not([data-processed])');
            if (unrenderedDiagrams.length > 0) {
                mermaid.init(undefined, unrenderedDiagrams);
            }
        }, 100);
    });

    // Simplest way to make mermaid re-render the diagrams in the new theme is via refreshing the page
    for (const darkTheme of darkThemes) {
        const themeBtn = document.getElementById(darkTheme);
        if (themeBtn) {
            themeBtn.addEventListener('click', () => {
                if (lastThemeWasLight) {
                    window.location.reload();
                }
            });
        }
    }

    for (const lightTheme of lightThemes) {
        const themeBtn = document.getElementById(lightTheme);
        if (themeBtn) {
            themeBtn.addEventListener('click', () => {
                if (!lastThemeWasLight) {
                    window.location.reload();
                }
            });
        }
    }
})();
