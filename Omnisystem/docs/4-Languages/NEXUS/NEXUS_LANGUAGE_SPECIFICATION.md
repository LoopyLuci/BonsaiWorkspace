# NEXUS Language Specification v1.0
## The Omnisystem Responsive Design Language

---

## 1. OVERVIEW

**NEXUS** replaces CSS, SCSS, Tailwind, layout engines. It provides:
- Constraint-based responsive layout (no media queries)
- Declarative design system
- Automatic grid/flex generation
- Theme composition
- Cross-platform design specs

---

## 2. CONSTRAINT-BASED LAYOUT

### 2.1 Layout Declarations

```nexus
// Container with size constraints
layout MainContainer {
    width: 100%
    height: auto
    padding: 20px
    gap: 16px
    direction: column
}

// Flexible item
item FlexItem {
    flex: 1
    min-width: 100px
    max-width: 500px
}

// Grid layout
layout GridContainer {
    display: grid
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr))
    gap: 24px
    width: 100%
}

// Responsive constraints
layout ResponsiveBox {
    width: auto
    constraints: {
        small: { width: 100%, padding: 8px },
        medium: { width: 80%, padding: 16px },
        large: { width: 60%, padding: 24px }
    }
}
```

### 2.2 Sizing & Spacing

```nexus
// Define spacing scale
spacing scale {
    xs: 4px,
    sm: 8px,
    md: 16px,
    lg: 24px,
    xl: 32px,
    2xl: 48px
}

// Use spacing
layout Card {
    padding: spacing.md
    margin-bottom: spacing.lg
    gap: spacing.sm
}

// Sizing
layout ImageContainer {
    width: 100%
    aspect-ratio: 16/9
    object-fit: cover
}
```

---

## 3. TYPOGRAPHY

### 3.1 Type System

```nexus
// Define typography scale
typography {
    display: { size: 48px, weight: bold, line-height: 1.2 },
    heading1: { size: 32px, weight: bold, line-height: 1.3 },
    heading2: { size: 24px, weight: 600, line-height: 1.4 },
    heading3: { size: 20px, weight: 600, line-height: 1.4 },
    body: { size: 16px, weight: 400, line-height: 1.5 },
    small: { size: 14px, weight: 400, line-height: 1.4 },
    tiny: { size: 12px, weight: 400, line-height: 1.3 }
}

// Apply typography
text Heading {
    apply: typography.heading1
    color: color.neutral-900
}

text Body {
    apply: typography.body
    color: color.neutral-600
}
```

---

## 4. COLOR SYSTEM

### 4.1 Color Palettes

```nexus
// Define color palette
colors {
    primary: {
        50: #f0f9ff,
        100: #e0f2fe,
        500: #0ea5e9,
        900: #0c2d4d
    },
    
    neutral: {
        0: #ffffff,
        50: #f9fafb,
        900: #111827
    },
    
    semantic: {
        success: #10b981,
        warning: #f59e0b,
        error: #ef4444,
        info: #3b82f6
    }
}

// Apply colors
element Button {
    background: colors.primary[500]
    color: colors.neutral[0]
    
    &:hover {
        background: colors.primary[600]
    }
}
```

---

## 5. COMPONENT LAYOUT

### 5.1 Component Composition

```nexus
// Button component
component Button {
    width: auto
    padding: spacing.md spacing.lg
    border-radius: 6px
    background: colors.primary[500]
    color: colors.neutral[0]
    cursor: pointer
    border: none
    
    transitions: {
        background: 200ms ease
    }
    
    states {
        disabled: {
            opacity: 0.5,
            cursor: not-allowed
        },
        hover: {
            background: colors.primary[600]
        },
        focus: {
            outline: 2px solid colors.primary[300]
        }
    }
}

// Card component
component Card {
    background: colors.neutral[0]
    border-radius: 8px
    padding: spacing.lg
    box-shadow: 0 1px 3px rgba(0,0,0,0.1)
    
    children: {
        header: { margin-bottom: spacing.md },
        body: { color: colors.neutral[600] },
        footer: { margin-top: spacing.md, border-top: 1px solid colors.neutral[200] }
    }
}
```

### 5.2 Responsive Components

```nexus
// Responsive navigation
component NavBar {
    display: flex
    direction: row
    gap: spacing.xl
    
    responsive {
        mobile (width < 640px): {
            direction: column,
            gap: spacing.md,
            width: 100%
        },
        tablet (640px <= width < 1024px): {
            gap: spacing.lg,
            width: 100%
        },
        desktop (width >= 1024px): {
            gap: spacing.xl,
            width: auto
        }
    }
}
```

---

## 6. THEME COMPOSITION

### 6.1 Theme Definitions

```nexus
// Light theme
theme light {
    colors: {
        background: colors.neutral[0],
        surface: colors.neutral[50],
        text: colors.neutral[900],
        border: colors.neutral[200],
        primary: colors.primary[500],
        error: colors.semantic.error
    }
}

// Dark theme
theme dark {
    colors: {
        background: colors.neutral[900],
        surface: colors.neutral[800],
        text: colors.neutral[0],
        border: colors.neutral[700],
        primary: colors.primary[400],
        error: colors.semantic.error
    }
}

// Theme switching
body {
    background: theme.background
    color: theme.text
    
    transition: background 200ms, color 200ms
}
```

---

## 7. ADVANCED LAYOUTS

### 7.1 Grid System

```nexus
// 12-column grid
layout GridSystem {
    display: grid
    grid-template-columns: repeat(12, 1fr)
    gap: spacing.md
    width: 100%
}

// Span columns
item GridItem-6 {
    grid-column: span 6
}

// Responsive grid
layout ResponsiveGrid {
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr))
    
    responsive {
        mobile: { grid-template-columns: 1fr },
        tablet: { grid-template-columns: repeat(2, 1fr) },
        desktop: { grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)) }
    }
}
```

### 7.2 Flexbox Layouts

```nexus
// Center content
layout CenterContent {
    display: flex
    align-items: center
    justify-content: center
    width: 100%
    height: 100%
}

// Space between items
layout SpaceBetween {
    display: flex
    justify-content: space-between
    align-items: center
    gap: spacing.md
}

// Wrap on small screens
layout FlexWrap {
    display: flex
    flex-wrap: wrap
    gap: spacing.md
    
    item: {
        flex: 0 1 auto,
        min-width: 200px
    }
}
```

---

## 8. EXAMPLE: RESPONSIVE DASHBOARD

```nexus
// Dashboard layout
layout Dashboard {
    display: grid
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr))
    gap: spacing.lg
    padding: spacing.xl
    width: 100%
    
    responsive {
        mobile (width < 640px): {
            grid-template-columns: 1fr,
            padding: spacing.md,
            gap: spacing.md
        },
        desktop (width >= 1024px): {
            grid-template-columns: repeat(4, 1fr),
            padding: spacing.xl
        }
    }
}

// Metric card
component MetricCard {
    apply: Card
    padding: spacing.lg
    min-height: 150px
    
    layout {
        display: flex
        direction: column
        gap: spacing.md
        width: 100%
        height: 100%
    }
}

// Header
layout DashboardHeader {
    display: flex
    justify-content: space-between
    align-items: center
    padding: spacing.lg
    border-bottom: 1px solid colors.neutral[200]
    margin-bottom: spacing.lg
    
    responsive {
        mobile: { direction: column, gap: spacing.md }
    }
}
```

---

This specification enables NEXUS to provide constraint-based, responsive design without media queries or CSS complexity.
