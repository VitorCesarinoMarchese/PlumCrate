---
name: Sims 4 Mod Manager website previews
description: Five marketing compositions extending the established Plumbob identity.
colors:
  ink: "#173452"
  muted: "#52687c"
  ground: "#edf4f8"
  surface: "#ffffff"
  line: "#d4e0e9"
  blue: "#1263ca"
  green: "#dfeccb"
  focus: "#2e5710"
  rail: "#0863b9"
  dark-ink: "#f4f8fb"
  dark-muted: "#bed0df"
  dark-ground: "#122c49"
  dark-surface: "#1d3a57"
  dark-line: "#405c75"
  dark-blue: "#9bc7ff"
  dark-focus: "#cceba2"
typography:
  display:
    fontFamily: "Manrope, sans-serif"
    fontSize: "clamp(52px, 5.4vw, 82px)"
    fontWeight: 700
    lineHeight: 1.04
    letterSpacing: "-0.04em"
  headline:
    fontFamily: "Manrope, sans-serif"
    fontSize: "clamp(32px, 3vw, 46px)"
    lineHeight: 1.13
    letterSpacing: "-0.03em"
  body:
    fontFamily: "Manrope, sans-serif"
    lineHeight: 1.75
  label:
    fontFamily: "Manrope, sans-serif"
    fontSize: "12px"
rounded:
  control: "6px"
  button: "9px"
  screenshot: "12px"
  section: "16px"
components:
  button-primary:
    backgroundColor: "{colors.blue}"
    textColor: "{colors.surface}"
    rounded: "{rounded.button}"
    padding: "16px 21px"
  selected-control:
    backgroundColor: "{colors.green}"
    textColor: "{colors.ink}"
    rounded: "{rounded.control}"
---

# Design System: Sims 4 Mod Manager website previews

## Overview

Plumbob remains the visual authority. This is a website-specific extension of [the native design system](../DESIGN.md), expressed through CSS and self-hosted Manrope. The original plumbob, blue navigation, pale blue surfaces, green selection and navy counterpart connect the marketing pages to the Linux app.

The owner selected Gallery for production. Home, Night, Guide and Studio remain accessible as optional alternatives through `?previews=1`. Their composition rules below describe implemented options. They do not replace native tokens or establish a new brand. `public/style.css`, `public/index.html` and `public/app.js` are the implementation sources; web dimensions use CSS pixels.

## Colors

Blue marks actions and headline emphasis. The rail blue supplies Guide navigation and Studio's opening field. Green identifies selected language, screenshot and composition controls. Pale blue ground, white sections, navy text and explicit muted text carry the default layouts.

Night uses the paired dark colors for its entire page and gives primary actions a pale blue fill with navy text. Studio uses white text and light green emphasis over its blue opening. Dark focus applies on the preview selector, Night, Guide's blue header, and Studio's blue header and hero. These are contrast-aware website applications of the inherited palette.

## Typography

Manrope regular and bold are self-hosted under the included font license. The default display and headline roles appear in the frontmatter. Body copy uses comfortable line spacing; introductory copy is 17px with a 36ch maximum, dropping to 16px on mobile. Supporting feature and question copy is 14px. Labels, screenshot controls, release status and captions use 12px. The brand pairs the original plumbob with a 17px bold name on desktop.

Each preview adjusts display size to its composition. Home uses `clamp(48px, 6.5vw, 92px)`, Night `clamp(55px, 6vw, 88px)`, Guide `clamp(48px, 5vw, 72px)`, and Studio `clamp(60px, 7.5vw, 96px)`. At the mobile breakpoint, headings use 62px except Night at 60px and Studio at 76px. Keep line wrapping legible in both English and Brazilian Portuguese.

## Layout

The shared page sequence is introduction and real app screenshot, capabilities, getting started, questions, and release links. Headers and default heroes have a 1440px maximum width with 6% horizontal padding; sections use a 1280px maximum and 95px vertical padding. Questions narrow to 960px. At 1000px spacing compresses. At 720px the hero and section grids stack, padding drops, navigation shortens, and the footer wraps.

- Gallery places introductory copy beside a larger screenshot in a 0.76fr/1.24fr grid.
- Home centers the introduction above a screenshot up to 1080px wide.
- Night divides introductory text into two columns above a dark screenshot up to 1040px wide, then stacks on mobile.
- Guide adds a sticky 225px blue index, reducing it to 185px below 1000px. At 720px it becomes ordinary top navigation.
- Studio places large typography and the screenshot on the blue opening field. Wide-screen padding keeps content centered; mobile stacks the two.

These are alternatives for the same content and identity. Gallery is selected for deployment. Follow [README.md](README.md) for the release configuration after selection.

## Elevation & Depth

Sections rely on background changes and thin dividers. The linked screenshot alone receives a soft shadow of `0 22px 48px #00000020`, darkening to `#0003` in Night. This website treatment frames an actual app capture; it is not a change to native card elevation.

## Shapes

Rounded controls, action buttons and screenshot frames use the frontmatter radii. The white getting-started section uses the section radius on desktop, becoming square on mobile and in Guide. Numbered setup steps use small circular green counters. Preserve the original authored plumbob rather than replacing it with a generated symbol.

## Components

Primary release actions have at least 48px height. The current-release status distinguishes lookup, an available AppImage, no published AppImage, and lookup failure. Available downloads must come from the actual repository's published release assets. Fallback links open GitHub releases. Release eligibility and verification commands live in README.md.

The preview selector changes composition and resets the screenshot to Light, or Dark for Night, while retaining the selected language. The URL stores preview and language. The production page hides the selector by default; `?previews=1` exposes it for comparison. Open clean preview hides it again. Selected buttons expose `aria-pressed`.

Light, Dark and Settings controls swap the genuine screenshot and its linked full-size image. Alt text follows the screenshot and language. The reveal lasts 0.35s with `cubic-bezier(.16, 1, .3, 1)`; reduced motion disables it and smooth scrolling. Captions disclose the demonstration library and link the original artwork credits.

EN and PT-BR update visitor copy, screenshot descriptions, accessible navigation labels, release text and the document language. Semantic question disclosures work without a custom accordion. A skip link and visible keyboard focus support navigation. Focus uses a 3px outline with 5px offset; the light green focus token applies against blue and dark backgrounds. Hover slightly reduces brightness.

### User guide

`public/guide.html`, `guide.css` and `guide.js` implement the Read layout on the existing white surface, with the established palette and self-hosted Manrope. The article is 16px with a base line height of 1.8; paragraphs retain the shared 1.75 rule. Introductory text is 18px, dropping to 17px on mobile. Heading sizes are 48px, 30px and 20px; the first two become 40px and 27px on mobile. Contents links and source disclosures use 13px, and screenshot captions use 12px.

The reading grid has a 205px sticky contents column, a 64px gap and an article up to 74ch wide within a 1200px container. Below 900px, the contents column narrows to 180px and the gap to 35px. Below 720px, contents and article stack, with the contents links arranged in two columns. Prose links remain underlined. The library illustration reuses the genuine app screenshot with its demonstration-library caption, creator credits and existing provenance.

## Do's and Don'ts

- Do inherit the native Plumbob identity while keeping website typography and layout scoped here.
- Do keep English and Brazilian Portuguese useful at desktop and mobile widths.
- Do use actual native captures with their demo disclosure, creator credits and provenance sidecars.
- Do retain truthful release availability and the Linux platform scope.
- Do keep Gallery as the selected production composition.
- Don't invent product UI, mod artwork, metrics, releases or generated visuals.
- Don't move website layout or CSS tokens into the native design authority.

## Reading position and persistent navigation

The site header stays at the top while scrolling. User guide remains a labeled navigation link on the landing page and guide, including small screens. The guide marks its page link with `aria-current="page"`.

Guide contents follow the section crossing the reading line below the sticky header. The current item uses the existing pale ground and muted text, a default cursor, `aria-current="location"` and `aria-disabled="true"`. Its click is ignored while current; the other links retain normal anchor navigation and visible keyboard focus. The final section is current at the document end. Selection updates after scrolling, resizing, image/layout changes and language changes without rewriting the URL hash.

The contents column sits 24px below the measured header height and scrolls internally in short desktop windows. Anchor and skip-link offsets account for the sticky header. Mobile contents stay in the existing inline grid.

Both pages render their header links from `public/site-navigation.js`. The shared order is The app, Get started, User guide, GitHub, with the same destinations and English/Portuguese labels. Static HTML fallback menus are checked against each other and the shared definition. Below 720px, both pages hide The app and Get started while retaining User guide and GitHub. Page context changes only the User guide current-page indication, never link order.
