/** Scoped styles: the generic reader never changes the surrounding admin page. */
export const calendarLoggerStyles = `
.calendar-logger{color:var(--logger-text);background:var(--logger-bg);border:1px solid var(--logger-border);border-radius:8px;overflow:hidden;min-width:0;font-family:var(--font-sans)}
.calendar-logger *{box-sizing:border-box}
.calendar-logger h2,.calendar-logger p{margin:0}
.calendar-logger small{font-size:14px}
.calendar-logger h2{font-size:15px;font-weight:650}
.calendar-logger button{min-height:44px;min-width:44px;font:inherit;color:inherit;cursor:pointer}
.calendar-logger button:disabled{opacity:.5;cursor:default}
.calendar-logger button:focus-visible,.calendar-logger [tabindex]:focus-visible{outline:2px solid var(--ff-foreground);outline-offset:-2px}
.calendar-logger .logger-header,.calendar-logger .logger-footer,.calendar-logger .tree-toolbar,.calendar-logger .reader-heading,.calendar-logger .reader-toolbar{display:flex;align-items:center;justify-content:space-between;gap:14px;padding:14px 16px;flex-wrap:wrap}
.calendar-logger .logger-header p{font:14px monospace;overflow-wrap:anywhere;opacity:.75;margin-top:6px}
.calendar-logger .logger-workspace{display:grid;grid-template-columns:var(--tree-width) 8px minmax(0,1fr);border-block:1px solid var(--logger-border);height:560px}
.calendar-logger .tree-panel,.calendar-logger .reader{min-width:0;min-height:0;display:flex;flex-direction:column}
.calendar-logger .tree-scroll{overflow:auto;flex:1;padding-block:6px}
.calendar-logger .tree-row{border:0;background:transparent;display:flex;align-items:center;gap:8px;width:100%;min-width:max-content;text-align:left;padding:7px 14px;font-size:14px}
.calendar-logger .tree-row:hover{background:var(--ff-background)}
.calendar-logger .tree-row.selected{background:var(--ff-background)}
.calendar-logger .entry-sizes{margin-left:auto;white-space:nowrap;font-size:14px;opacity:.7}
.calendar-logger .root-sizes{display:flex;flex-direction:column;gap:4px;padding:8px 14px;border-bottom:1px solid var(--logger-border);font-size:14px;opacity:.8}
.calendar-logger .chevron{width:14px}
.calendar-logger .entry-name{white-space:nowrap}
.calendar-logger .root-caption,.calendar-logger .tree-message,.calendar-logger .more-tree{padding:14px;font-size:14px;opacity:.8}
.calendar-logger .logger-divider{display:flex;align-items:center;justify-content:center;cursor:col-resize;touch-action:none;font-size:14px;border-inline:1px solid var(--logger-border)}
.calendar-logger .logger-action{border:1px solid var(--logger-border);border-radius:6px;background:transparent;padding:7px 14px;font-size:14px;white-space:nowrap}
.calendar-logger .reader-heading{border-bottom:1px solid var(--logger-border)}
.calendar-logger .file-actions{display:flex;align-items:center;gap:8px;flex-wrap:wrap;padding:8px 16px;border-bottom:1px solid var(--logger-border);font-size:14px}
.calendar-logger .delete-confirm{max-width:min(520px,calc(100vw - 32px));color:var(--logger-text);background:var(--logger-bg);border:1px solid var(--logger-border);border-radius:8px;padding:20px}
.calendar-logger .delete-confirm::backdrop{background:var(--ff-overlay)}
.calendar-logger .delete-confirm p{overflow-wrap:anywhere;margin:0 0 14px}
.calendar-logger .delete-confirm .logger-action{margin-right:8px}
.calendar-logger .reader-heading h2{overflow-wrap:anywhere}
.calendar-logger .reader-toolbar,.calendar-logger .connection,.calendar-logger .logger-footer{font-size:14px}
.calendar-logger .connection.live{color:var(--ff-foreground)}
.calendar-logger .reader-alert{padding:14px 16px;background:var(--ff-background);font-size:14px}
.calendar-logger .window-note{padding:6px 16px;opacity:.7;font-size:14px}
.calendar-logger .log-scroll{flex:1;overflow:auto;min-height:0;background:var(--ff-background);position:relative;overscroll-behavior:contain}
.calendar-logger .log-canvas{position:relative}
.calendar-logger .log-lines{position:absolute;margin:0;padding:0 16px;font:14px/22px ui-monospace,SFMono-Regular,Menlo,monospace;white-space:pre;user-select:text}
.calendar-logger .reader-empty{padding:24px;font-size:14px;opacity:.65}
@media(max-width:760px){.calendar-logger .logger-workspace{grid-template-columns:minmax(0,1fr);grid-template-rows:200px minmax(0,1fr);height:700px}.calendar-logger .logger-divider{display:none}.calendar-logger .tree-panel{border-bottom:1px solid var(--logger-border)}.calendar-logger .entry-sizes{display:none}}
`;
