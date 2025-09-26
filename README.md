# Cosmarium

**Cosmarium** is a next-generation creative writing software designed for fiction authors (novels, short stories, novellas, long-form series) who want power, modularity, speed, and immersion.
Inspired by the best development (Zed, VS Code) and writing tools (FocusWriter, Manuskript, Scrivener), Cosmarium puts modularity, ergonomics, and openness at the heart of its architecture.

---

## ✨ Vision

- **Write without friction**: a lightweight, fast, immersive interface that disappears in front of your text.
- **Organize and connect everything**: entities, timelines, places, objects, notes, goals, versions… everything is accessible, structured, and interconnected.
- **Total modularity**: every feature is a module, enable/disable as you wish, extensible via plugins.
- **Open to AI and modern tools**: narrative assistants, style analysis, sound and graphic generation, MuseTag integration.
- **Collaboration and community**: write solo or together, share, discuss, and grow as a group.

---

## 🚀 Main Features

- **Immersive markdown editor** (inspired by Zed/FocusWriter) with optional preview.
- **Modular panels**: entities (characters, places, objects…), timeline, notes, goals, statistics, etc.
- **Integrated versioning and branching** (git-like, but internal and transparent).
- **Project = compressed file or structured folder** (user choice).
- **Export**: Markdown, PDF, TXT, HTML, Word, LaTeX.
- **Advanced statistics**: quantity, quality, style analysis, word frequency, sentence length, etc.
- **Hemingway-style analysis**: readability grade, difficult sentences detection, weakener words, complex word alternatives.
- **Writing goals**: progress tracking, notifications, history.
- **Immersive mode**: configurable panels, AI-generated soundscape, dynamic graphic backgrounds.
- **Conversational AI panel**: discuss your story, unlock inspiration, structure your plot.
- **Plugin architecture**: narrative assistants (Snowflake method, narrative templates…), analysis tools, community extensions.
- **Real-time collaboration**: co-writing, comments, access rights management.
- **Community panels**: forums, thematic chat rooms, project sharing.

---

## 🗺️ Roadmap

### v0.1.0 – MVP (Miniam Viable Product)

**Phase 1: Core Foundation**
- [ ] Project management (create/open/save compressed file or folder)
- [ ] Panel infrastructure (layout, resize, shortcuts - but empty panels)
- [ ] Basic immersive writing mode
- For dev:
  - [ ] Ultra-minimal EGUI core (window, layout manager only)
  - [ ] Plugin system foundation (registry, loading, API)
  - [ ] Document abstraction layer
  - [ ] Plugin: Markdown editor (as first reference plugin)
  - [ ] Workspace structure (core + plugin-api + plugins crates)

**Phase 2: Modular Architecture**
- [ ] Plugin: Notes panel
- [ ] Plugin: Manual entities panel
- [ ] Plugin: Outliner/project tree panel
- For dev:
  - [ ] Plugin development kit (macros, helpers)
  - [ ] Hot-reload during development
  - [ ] Inter-plugin communication system
  - [ ] Plugin template/scaffolding

**Phase 3: MVP Completion**
- [ ] Plugin: Export (PDF, Markdown, TXT)
- [ ] Plugin: Simple writing goals (word count targets, progress tracking)
- For dev:
  - [ ] Plugin API validation with real use cases
  - [ ] Documentation for plugin development
  - [ ] CI/CD for plugin ecosystem

### v0.2.x – Differentiation (What Makes Cosmarium Unique)
- [ ] MuseTag integration preparation (entity structure, timeline support)
- [ ] AI soundscape generation (SoundVault/freesound-rs integration)
- [ ] AI graphic generation for entities/places (backgrounds, character portraits)
- [ ] Dynamic visual adaptation based on content being written
- For dev:
  - [ ] Multi-language plugin support foundation
  - [ ] Lua plugin support (mlua) - simplest embeddable scripting

### v0.3.x – Innovation (Original Implementation)
- [ ] Conversational AI panel (writing assistant)
- [ ] Multiple AI personalities (editor, critic, coach, fan, skeptic, etc.)
- [ ] Reference author personalities ("What would Tolkien/King/Atwood do?")
- [ ] Character AI conversations (chat with your story entities)
- [ ] Multi-character "round table" discussions
- For dev:
  - [ ] JavaScript runtime integration (deno_core/rquickjs) - web ecosystem access

### v0.4.x – Standard Features (Expected Functionality)
- [ ] Advanced statistics (quantitative and qualitative analysis)
- [ ] Hemingway-inspired style analysis (readability, difficult sentences, weakeners)
- [ ] Write/Edit/Feedback modes with visual highlighting
- [ ] Integrated versioning/branching (git-like, graphical interface)
- [ ] Export to HTML, Word, LaTeX
- [ ] Import from existing projects (Markdown, Manuskript, Scrivener)
- For dev:
  - [ ] Python plugin support (PyO3) - AI/ML ecosystem integration

### v0.5.x – Professional Features
- [ ] Narrative templates and story structure assistants (Snowflake method, etc.)
- [ ] Advanced writing goals and progress analytics
- [ ] Multi-device synchronization
- [ ] Advanced accessibility (themes, keyboard navigation)

### v0.6.x – Collaboration & Community
- [ ] Real-time collaboration (co-writing, comments, access rights)
- [ ] Community panels (forums, thematic chat rooms, project sharing)
- [ ] Collaborative editing and review workflows

### v1.0.0 – First Stable Release
- [ ] Complete documentation (user, plugin API)
- [ ] Standard plugin library (Snowflake, MuseTag, narrative analysis, advanced export)
- [ ] Community ecosystem (extensions marketplace, contributions)
- For dev:
  - [ ] WASM plugin support (wasmtime)
  - [ ] Plugin marketplace infrastructure
  - [ ] Automated tests, CI/CD, multi-platform installers
  - [ ] Plugin security/sandboxing model

---

## 📚 Inspirations & Related Projects

- [Zed](https://zed.dev/): ergonomics, panel management, speed, collaboration.
- [MuseTag](https://musetag.github.io/): semantic annotation, entity management, timeline.
- [Manuskript](https://www.theologeek.ch/manuskript/): project structuring, entity management, outliner.
- [FocusWriter](https://gottcode.org/focuswriter/): immersive mode, writing goals.
- [SoundVault](https://github.com/taophp/soundvault): soundscape generation.
- [freesound-rs](https://github.com/taophp/freesound-rs): soundscape generation.
- [Snowflake Method](http://www.advancedfictionwriting.com/articles/snowflake-method/): project structuring.

---

## 💡 Want to contribute?

Cosmarium is under active development. Any help, suggestion, or feedback is welcome!
Join us to help shape the writing tool of tomorrow.
