#[derive(Debug, Clone, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum ResolveKind {
    /// entry input in the config
    Entry(String),
    /// static import, e.g. `import a from './a'`
    #[default]
    Import,
    /// static export, e.g. `export * from './a'`
    ExportFrom,
    /// dynamic import, e.g. `import('./a').then(module => console.log(module))`
    DynamicImport,
    /// cjs require, e.g. `require('./a')`
    Require,
    /// @import of css, e.g. @import './a.css'
    CssAtImport,
    /// url() of css, e.g. url('./a.png')
    CssUrl,
    /// `<script src="./index.html" />` of html
    ScriptSrc,
    /// `<link href="index.css" />` of html
    LinkHref,
    /// Hmr update
    HmrUpdate,
    /// Custom ResolveKind, e.g. `const worker = new Worker(new Url("worker.js"))` of a web worker
    Custom(String),
}
