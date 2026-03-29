# Wispr Flow Capability Analysis

Date: 2026-03-29
Scope: Detailed capability breakdown of Wispr Flow (with emphasis on "whisper" usage, dictation, developer workflow, and security/compliance).

## Whisper Capability
- Supports quiet/whisper-level dictation for low-noise environments.
- Not a separate product tier; part of normal dictation behavior.

## Core Dictation
- Dictation in most text fields and apps (documents, chat, email, IDEs, browser forms).
- Real-time transcription with auto-paste behavior.
- 100+ language support.
- Platform coverage: Mac, Windows, iOS, Android.
- Public claim: roughly 4x typing speed.

## Live Text Refinement
- Backtracking correction handling while speaking.
- Filler-word cleanup.
- Automatic punctuation based on speech.
- Spoken list formatting.
- Custom dictionary learning.
- Snippets (voice shortcuts for longer text).
- Style/tone adaptation features (desktop English focus in current docs).

## Command Mode
- Voice-driven rewrite/transform of selected text.
- Actions include translation, summarization, expansion, and tone edits.
- Query mode can route to web search (Perplexity) when no text is selected.
- Keyboard-triggered flow with escape/cancel and undo support.
- Selection window in current docs: 1-1000 words.
- iOS support is partial (search prompts supported; text-edit behavior has limits).

## Developer Capabilities
- IDE-oriented dictation in Cursor, VS Code, Windsurf, and terminals.
- Variable recognition from visible editor context.
- Supported languages listed: JavaScript, TypeScript, Python, Java, Swift, C++, C, Rust, Go.
- File tagging for chat workflows in Cursor and Windsurf (not VS Code chat tagging).
- Terminal caveat: Flow uses standard paste shortcuts; fallback is "Paste last transcript" or manual paste.
- VS Code Insiders currently has stated limits for variable recognition/file tagging.

## Team and Admin Features
- Shared dictionary and shared snippets.
- Team usage dashboards.
- Centralized billing and admin controls.
- Enterprise-tier admin controls include SSO/SAML and additional governance controls.

## Security, Privacy, and Compliance
- Privacy Mode / Zero Data Retention for dictation content.
- Enterprise can enforce ZDR organization-wide.
- HIPAA BAA support with privacy-lock behavior once BAA/ZDR is enforced.
- SOC 2 Type II and ISO 27001 claims in compliance documentation.
- Encryption in transit (TLS) and encryption-at-rest controls.
- Important nuance: dictation content protections differ from operational metadata retention (usage/account/billing/session telemetry).

## Plan-Level Positioning (High Level)
- Basic: capped usage plus core dictation, dictionary/snippets/languages/privacy mode.
- Pro: unlimited usage plus Command Mode and advanced/productivity features.
- Enterprise: compliance/admin/security controls and enterprise support.

## Operational Notes for ASR Bridge Context
- Wispr Flow is optimized for direct text insertion into active apps and can be used with terminal workflows via paste fallbacks.
- For Codex/terminal scenarios, reliability depends on focus state and terminal paste behavior.
- Compared to a custom ASR bridge, Wispr Flow reduces implementation overhead but introduces vendor/runtime dependency and app-level integration constraints.

## Sources
- https://wisprflow.ai/features
- https://wisprflow.ai/pricing
- https://docs.wisprflow.ai/articles/2772472373-what-is-flow
- https://docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode
- https://docs.wisprflow.ai/articles/6434410694-use-flow-with-cursor-vs-code-and-other-ides
- https://docs.wisprflow.ai/articles/9559327591-flow-plans-and-what-s-included
- https://docs.wisprflow.ai/articles/6274675613-privacy-mode-data-retention
- https://docs.wisprflow.ai/articles/6939510703-compliance-certifications-standards
- https://docs.wisprflow.ai/articles/1922179110-data-security-encryption
- https://docs.wisprflow.ai/articles/5375461355-subprocessors-third-party-security
- https://trust.wispr.ai/
