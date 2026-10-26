#!/usr/bin/env python3
"""Reproduce an UNAPPLIED patch from exact git-show fixtures; never edit host."""
from pathlib import Path
import difflib
ROOT=Path(__file__).resolve().parents[1]
BASE=ROOT/'qa/fixtures/host'
OUT=ROOT/'.qa/host-patched'
changes={}
def edit(path,fn):
 old=(BASE/path).read_text();new=fn(old);assert new!=old,path;changes[path]=(old,new)
def replace(text,old,new):
 assert text.count(old)==1,(old,text.count(old));return text.replace(old,new)
p='src/renderer/src/raycast-api/form-runtime.tsx'
def form(s):
 s=replace(s,'  actionId?: string\n','  actionId?: string\n  nativeAbsolutePath?: boolean\n')
 s=replace(s,'          allowMultipleSelection: node.props?.allowMultipleSelection === true,','          allowMultipleSelection: node.props?.allowMultipleSelection === true,\n          nativeAbsolutePath: node.props?.nativeAbsolutePath === true,')
 s=replace(s,'onChangeField: (actionId: string, value: unknown) => Promise<void> | void','onChangeField: (actionId: string, value: unknown, values?: Record<string, unknown>) => Promise<void> | void')
 marker="  const nativeColorWheel = useMemo"
 s=s.replace(marker,"""  const formDraft = useRef({ token: root.props?.formToken, revision: Number(root.props?.formRevision ?? 0), values })
  const pickerAlive = useRef(true)
  const pickerRequest = useRef(0)
  const [pickerError, setPickerError] = useState('')
  useEffect(() => { pickerAlive.current = true; return () => { pickerAlive.current = false } }, [])
  const snapshot = (): Record<string, unknown> => typeof root.props?.formToken === 'string'
    ? { ...formDraft.current.values, __formToken: root.props.formToken, __formRevision: formDraft.current.revision }
    : formDraft.current.values
"""+marker)
 s=replace(s,'    if (nativeColorWheel) {\n      localWheelCoordinates',"""    const token = root.props?.formToken
    const revision = Number(root.props?.formRevision ?? 0)
    if (typeof token === 'string' && formDraft.current.token === token && revision < formDraft.current.revision) return
    if (formDraft.current.token !== token) {
      formDraft.current = { token, revision, values: reconcileFormValues(fields) }
    }
    if (nativeColorWheel) {
      localWheelCoordinates""")
 s=replace(s,'    setValues((previous) => ({\n      ...reconcileFormValues(fields, previousFields ? previous : undefined, previousFields),\n      ...wheelDraft,\n    }))',"""    const next = { ...reconcileFormValues(fields, previousFields ? formDraft.current.values : undefined, previousFields), ...wheelDraft }
    formDraft.current = { token, revision, values: next }
    setValues(next)""")
 s=replace(s,'  }, [fields, nativeColorWheel, title])','  }, [fields, nativeColorWheel, title, root.props?.formToken, root.props?.formRevision])')
 s=replace(s,'useImperativeHandle(ref, () => ({ getValues: () => values }), [values])','useImperativeHandle(ref, () => ({ getValues: snapshot }))')
 s=replace(s,'    setValues((previous) => ({ ...previous, [field.id]: value }))\n    if (field.actionId) void onChangeField(field.actionId, value)',"""    const next = { ...formDraft.current.values, [field.id]: value }
    formDraft.current = { ...formDraft.current, revision: formDraft.current.revision + 1, values: next }
    setValues(next)
    if (field.actionId) void onChangeField(field.actionId, value, snapshot())""")
 s=s.replace('onSubmitForm(values)','onSubmitForm(snapshot())')
 marker="            if (field.type === 'Form.FilePicker') {"
 s=replace(s,marker,"""            if (field.type === 'Form.FilePicker' && field.nativeAbsolutePath) {
              const selected = Array.isArray(rawValue) ? rawValue.filter((v): v is string => typeof v === 'string') : []
              return <div key={field.key} className={FIELD_ROW_CLASS}>
                <div className={FIELD_TITLE_CLASS}>{field.title}</div>
                <div><button type="button" className="glass-field" onClick={() => {
                  const token = root.props?.formToken
                  const request = ++pickerRequest.current
                  const revision = formDraft.current.revision
                  setPickerError('')
                  void window.tezbar.pickExtensionFile().then((path) => {
                    if (!pickerAlive.current || formDraft.current.token !== token || request !== pickerRequest.current || revision !== formDraft.current.revision) return
                    if (path !== null) updateValue(field, [path])
                  }).catch(() => { if (pickerAlive.current && request === pickerRequest.current && formDraft.current.token === token) setPickerError('Could not choose file') })
                }}>Choose File</button>
                  {selected.map((path) => <span key={path}>{displayFileName(path)}</span>)}
                  {selected.length ? <button type="button" onClick={() => { pickerRequest.current++; updateValue(field, []) }}>Clear</button> : null}
                  {pickerError ? <span role="alert">{pickerError}</span> : null}
                  <FormFieldFeedback field={field} />
                </div>
              </div>
            }
"""+marker)
 return s
edit(p,form)
edit('src/renderer/src/raycast-api/index.tsx',lambda s:replace(s,'onChangeField={(actionId, value) => onInvokeAction(actionId, { value })}','onChangeField={(actionId, value, values) => onInvokeAction(actionId, values ?? { value })}'))
edit('src/renderer/src/raycast-api/detail-runtime.tsx',lambda s:replace(s,'onClick={onRunPrimaryAction}','onClick={() => onRunPrimaryAction()}'))
def view(s):
 s=replace(s,'  const searchRequestSeq = useRef(0)','  const searchRequestSeq = useRef(0)\n  const actionRequestSeq = useRef(0)')
 s=replace(s,'              setError(null)\n              const invokedAction','              const actionSeq = ++actionRequestSeq.current\n              setError(null)\n              const invokedAction')
 s=replace(s,'              if (currentSession.current !== sessionOwner) return','              if (currentSession.current !== sessionOwner || actionSeq !== actionRequestSeq.current) return\n              if (result.ok && result.mode === \'unchanged\') return')
 s=replace(s,'          const result = await window.tezbar.extensionRefreshSession','          const actionSeq = actionRequestSeq.current\n          const result = await window.tezbar.extensionRefreshSession')
 s=replace(s,"          if (cancelled || (result.ok && result.mode === 'unchanged')) return","          if (cancelled || actionSeq !== actionRequestSeq.current || (result.ok && result.mode === 'unchanged')) return")
 return s
edit('src/renderer/ExtensionRuntimeView.tsx',view)
edit('src/shared/desktop-api.ts',lambda s:replace(s,'  clipboardReadText:', '  pickExtensionFile: () => Promise<string | null>\n  clipboardReadText:'))
def bridge(s):
 return replace(s,"    clipboardReadText: () => callBackend('clipboard:read'),","""    pickExtensionFile: async () => {
      await setNativeBlurHideSuppressed(true)
      try { return await invoke<string | null>('pick_extension_file') }
      finally { await setNativeBlurHideSuppressed(false).catch(() => undefined) }
    },
    clipboardReadText: () => callBackend('clipboard:read'),""")
edit('src/renderer/tauri-bridge.ts',bridge)
edit('src-tauri/src/lib.rs',lambda s:replace(replace(s,'mod qr_download;','mod qr_download;\nmod extension_file_picker;'),'            qr_download::save_qr_png_data_url','            extension_file_picker::pick_extension_file,\n            qr_download::save_qr_png_data_url'))
changes['src-tauri/src/extension_file_picker.rs']=('', '''// Generic explicit single-file selection. No extension-specific route or scan.
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

#[tauri::command]
pub async fn pick_extension_file(app: AppHandle) -> Result<Option<String>, String> {
    let (sender, receiver) = oneshot::channel();
    app.dialog().file().pick_file(move |path| { let _ = sender.send(path); });
    let Some(path) = receiver.await.map_err(|_| "Could not open file dialog")? else { return Ok(None); };
    let path = path.into_path().map_err(|_| "Could not resolve selected file")?;
    if !path.is_absolute() || !std::fs::symlink_metadata(&path).map_err(|e| e.to_string())?.is_file() {
        return Err("Select one ordinary absolute file".into());
    }
    let path = path.to_str().ok_or("Selected path must be UTF-8")?;
    if path.len() > 4096 { return Err("Selected path is too long".into()); }
    Ok(Some(path.into()))
}
''')
# Keep the canonical preference path, while resolving all resources from the
# independently verified active version. No extension-specific lookup table.
def install(s):
 old = '    let root = root(ctx, id)?;\n    let active = read_json'
 new = '    resolve_at(root(ctx, id)?, id)\n}\npub(super) fn resolve_at(root: PathBuf, id: &str) -> Result<Package, String> {\n    for path in [&root, &root.join("versions")] {\n        let m = std::fs::symlink_metadata(path).map_err(|e| e.to_string())?;\n        if !m.is_dir() || m.file_type().is_symlink() { return Err("Package root symlink/non-directory rejected".into()); }\n    }\n    let active = read_json'
 return replace(s, old, new)
edit('src-tauri/src/native_core/extensions/process_install.rs', install)
def registry(s):
 start=s.index('    let native = if id.starts_with("raycast.")')
 end=s.index('    if let Some(extension) = native {', start)
 s=s[:start]+'''    let mut matches = collect(ctx).into_iter().filter(|entry| {
        entry.id.starts_with("com.tezbar.native.")
            && entry.manifest["healthError"].is_null()
            && (entry.id == id || entry.manifest["replaces"].as_array()
                .is_some_and(|ids| ids.iter().any(|alias| alias.as_str() == Some(id))))
    });
    let native = matches.next();
    if matches.next().is_some() { return None; } // Ambiguous publisher claims fail closed.
'''+s[end:]
 s=replace(s, '        let active = extension.manifest["healthError"].is_null()', '        let active = compatible(&extension.manifest, false) && extension.manifest["healthError"].is_null()')
 helper='''fn resource_root(extension: &Installed) -> Option<PathBuf> {
    if extension.id.starts_with("com.tezbar.native.") {
        super::process_install::resolve_at(extension.path.clone(), &extension.id).ok().map(|p| p.dir)
    } else { Some(extension.path.clone()) }
}

'''
 s=replace(s,'fn icon_path(extension: &Installed) -> Option<String> {', helper+'fn icon_path(extension: &Installed) -> Option<String> {')
 # Two identical icon candidate blocks, both must switch to verified resources.
 old='    let mut candidates = vec![\n        extension.path.join(relative),\n        extension.path.join("assets").join(relative),\n    ];'
 assert s.count(old)==2
 s=s.replace(old,'    let resources = resource_root(extension)?;\n    let mut candidates = vec![resources.join(relative), resources.join("assets").join(relative)];')
 return s
edit('src-tauri/src/native_core/extensions/registry.rs',registry)
def native(s):
 s=replace(s,'requested.starts_with("com.tezbar.native.") || v2_runtime::slug(requested).is_some()', 'requested.starts_with("com.tezbar.native.") || registry::native_command_id(ctx, requested, body["commandName"].as_str().unwrap_or("")).is_some()')
 s=replace(s,'            v2_runtime::canonical_id(v2_runtime::slug(requested).ok_or("Unknown extension")?)', '            match registry::native_command_id(ctx, requested, command) { Some(id) => id, None => return Ok(None) }')
 return s
edit('src-tauri/src/native_core/extensions/native.rs',native)
p='src-tauri/src/native_core/extensions/registry.rs'
old,new=changes[p]
marker='fn platform()'
new=new.replace(marker,'''pub(super) fn native_command_id(ctx: &CoreContext, id: &str, command: &str) -> Option<String> {
    installed_command(ctx, id, command).filter(|e| e.id.starts_with("com.tezbar.native.")).map(|e| e.id)
}

'''+marker,1)
changes[p]=(old,new)
# Tokenized native forms freeze controls only while a deliberate exposed action
# is pending. Draft changes remain available between submits; a late picker must
# never change the image underneath a pending upload.
p='src/renderer/src/raycast-api/index.tsx'
old,new=changes[p]
new=replace(new,'    onInvokeAction,\n  } = props','    onInvokeAction: invokeAction,\n  } = props')
marker='  const [showActions, setShowActions] = useState(false)'
new=replace(new,marker,"""  const pendingAction = useRef(false)
  const [formPending, setFormPending] = useState(false)
  const onInvokeAction = useCallback(async (id: string, values?: Record<string, unknown>) => {
    const tokenized = root.type === 'Form' && typeof root.props?.formToken === 'string'
    if (tokenized && pendingAction.current) return
    const effect = tokenized && actions.some(action => action.id === id)
    if (effect) { pendingAction.current = true; setFormPending(true) }
    try { await invokeAction(id, values) }
    finally { if (effect) { pendingAction.current = false; setFormPending(false) } }
  }, [invokeAction, root, actions])
"""+marker)
new=replace(new,'              ref={formRuntimeRef}','              ref={formRuntimeRef}\n              pending={formPending}')
changes[p]=(old,new)
p='src/renderer/src/raycast-api/form-runtime.tsx'
old,new=changes[p]
new=replace(new,'  onSubmitForm: (values:', '  pending?: boolean\n  onSubmitForm: (values:')
new=replace(new,'{ root, title, onBack, onSubmitForm, onChangeField, onInvokeAction, onOpenActions, actionNotice }','{ root, title, onBack, onSubmitForm, onChangeField, onInvokeAction, onOpenActions, actionNotice, pending = false }')
new=replace(new,'  const pickerAlive = useRef(true)','  const pendingRef = useRef(pending)\n  pendingRef.current = pending\n  const pickerAlive = useRef(true)')
new=replace(new,'  const updateValue = (field: FormField, value: unknown): void => {','  const updateValue = (field: FormField, value: unknown): void => {\n    if (pendingRef.current) return')
new=new.replace('        onSubmitForm(snapshot())','        if (!pendingRef.current) onSubmitForm(snapshot())')
new=replace(new,'        <div className="mx-auto max-w-[900px] space-y-4">','        <fieldset disabled={pending} className="mx-auto max-w-[900px] space-y-4 border-0 p-0 m-0 min-w-0">')
new=replace(new,'        </div>\n      </div>\n\n      {actionNotice','        </fieldset>\n      </div>\n\n      {actionNotice')
new=replace(new,"if (!pickerAlive.current || formDraft.current.token", "if (pendingRef.current || !pickerAlive.current || formDraft.current.token")
changes[p]=(old,new)
# Plain literal Detail content uses React escaping and has no separate Markdown
# toolbar copy path, preserving the extension-owned exact-text ACK workflow.
p='src/renderer/src/raycast-api/detail-runtime.tsx'
old,new=changes[p]
new=replace(new,"          {imageOnly && imageOnly.alt.toLowerCase().includes('color wheel') ? (", """          {typeof root.props?.literalText === 'string' ? (
            <pre dir="ltr" style={{ unicodeBidi: 'isolate' }} className="whitespace-pre overflow-auto text-[13px] leading-relaxed"><code>{root.props.literalText}</code></pre>
          ) : imageOnly && imageOnly.alt.toLowerCase().includes('color wheel') ? (""")
changes[p]=(old,new)
patch=''
for path,(old,new) in changes.items():
 d=OUT/path;d.parent.mkdir(parents=True,exist_ok=True);d.write_text(new)
 patch+=''.join(difflib.unified_diff(old.splitlines(keepends=True),new.splitlines(keepends=True),fromfile='a/'+path if old else '/dev/null',tofile='b/'+path))
(ROOT/'integration/host-prerequisites.patch').write_text(patch)
print(f'UNAPPLIED host patch: {len(changes)} files, {len(patch)} bytes; source pin ea100cee')
