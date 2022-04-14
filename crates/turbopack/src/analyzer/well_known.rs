use std::{mem::take, sync::Arc};

use url::Url;

use super::{ConstantValue, JsValue, WellKnownFunctionKind, WellKnownObjectKind};

pub fn replace_well_known(value: JsValue) -> (JsValue, bool) {
    match value {
        JsValue::Call(_, box JsValue::WellKnownFunction(kind), args) => (
            well_known_function_call(
                kind,
                JsValue::Unknown(None, "this is not analysed yet"),
                args,
            ),
            true,
        ),
        JsValue::Member(_, box JsValue::WellKnownObject(kind), box prop) => {
            (well_known_object_member(kind, prop), true)
        }
        JsValue::Member(_, box JsValue::WellKnownFunction(kind), box prop) => {
            (well_known_function_member(kind, prop), true)
        }
        _ => (value, false),
    }
}

pub fn well_known_function_call(
    kind: WellKnownFunctionKind,
    _this: JsValue,
    args: Vec<JsValue>,
) -> JsValue {
    match kind {
        WellKnownFunctionKind::PathJoin => path_join(args),
        WellKnownFunctionKind::PathDirname => path_dirname(args),
        WellKnownFunctionKind::Import => JsValue::Unknown(
            Some(Arc::new(JsValue::call(
                box JsValue::WellKnownFunction(kind),
                args,
            ))),
            "import() is not supported",
        ),
        WellKnownFunctionKind::Require => require(args),
        WellKnownFunctionKind::RequireResolve => JsValue::Unknown(
            Some(Arc::new(JsValue::call(
                box JsValue::WellKnownFunction(kind),
                args,
            ))),
            "require.resolve() is not supported",
        ),
        WellKnownFunctionKind::PathToFileUrl => path_to_file_url(args),
        _ => JsValue::Unknown(
            Some(Arc::new(JsValue::call(
                box JsValue::WellKnownFunction(kind),
                args,
            ))),
            "unsupported function",
        ),
    }
}

pub fn path_join(args: Vec<JsValue>) -> JsValue {
    if args.len() == 0 {
        return ".".into();
    }
    let mut parts = Vec::new();
    for item in args {
        if let Some(str) = item.as_str() {
            let splitted = str.split("/");
            parts.extend(splitted.map(|s| s.into()));
        } else {
            parts.push(item);
        }
    }
    let mut results_final = Vec::new();
    let mut results: Vec<JsValue> = Vec::new();
    for item in parts {
        if let Some(str) = item.as_str() {
            match str {
                "" | "." => {
                    if results_final.is_empty() && results.is_empty() {
                        results_final.push(item);
                    }
                }
                ".." => {
                    if results.pop().is_none() {
                        results_final.push(item);
                    }
                }
                _ => results.push(item),
            }
        } else {
            results_final.extend(results.drain(..));
            results_final.push(item);
        }
    }
    results_final.extend(results.drain(..));
    let mut iter = results_final.into_iter();
    let first = iter.next().unwrap();
    let mut last_is_str = first.as_str().is_some();
    results.push(first);
    for part in iter {
        let is_str = part.as_str().is_some();
        if last_is_str && is_str {
            results.push("/".into());
        } else {
            results.push(JsValue::alternatives(vec!["/".into(), "".into()]));
        }
        results.push(part);
        last_is_str = is_str;
    }
    JsValue::concat(results)
}

pub fn path_dirname(mut args: Vec<JsValue>) -> JsValue {
    if let Some(arg) = args.iter_mut().next() {
        if let Some(str) = arg.as_str() {
            if let Some(i) = str.rfind("/") {
                return JsValue::Constant(ConstantValue::Str(str[..i].to_string().into()));
            } else {
                return JsValue::Constant(ConstantValue::Str("".into()));
            }
        } else if let JsValue::Concat(_, items) = arg {
            if let Some(last) = items.last_mut() {
                if let Some(str) = last.as_str() {
                    if let Some(i) = str.rfind("/") {
                        *last = JsValue::Constant(ConstantValue::Str(str[..i].to_string().into()));
                        return take(arg);
                    }
                }
            }
        }
    }
    JsValue::Unknown(
        Some(Arc::new(JsValue::call(
            box JsValue::WellKnownFunction(WellKnownFunctionKind::PathDirname),
            args,
        ))),
        "path.dirname with unsupported arguments",
    )
}

pub fn require(args: Vec<JsValue>) -> JsValue {
    if args.len() == 1 {
        match &args[0] {
            JsValue::Constant(ConstantValue::Str(s)) => JsValue::Module(s.clone()),
            _ => JsValue::Unknown(
                Some(Arc::new(JsValue::call(
                    box JsValue::WellKnownFunction(WellKnownFunctionKind::Require),
                    args,
                ))),
                "only constant argument is supported",
            ),
        }
    } else {
        JsValue::Unknown(
            Some(Arc::new(JsValue::call(
                box JsValue::WellKnownFunction(WellKnownFunctionKind::Require),
                args,
            ))),
            "only a single argument is supported",
        )
    }
}

pub fn path_to_file_url(args: Vec<JsValue>) -> JsValue {
    if args.len() == 1 {
        match &args[0] {
            JsValue::Constant(ConstantValue::Str(path)) => Url::from_file_path(&**path)
                .map(JsValue::Url)
                .unwrap_or_else(|_err| {
                    JsValue::Unknown(
                        Some(Arc::new(JsValue::call(
                            box JsValue::WellKnownFunction(WellKnownFunctionKind::PathToFileUrl),
                            args,
                        ))),
                        // TODO include err in message
                        "url not parseable",
                    )
                }),
            _ => JsValue::Unknown(
                Some(Arc::new(JsValue::call(
                    box JsValue::WellKnownFunction(WellKnownFunctionKind::PathToFileUrl),
                    args,
                ))),
                "only constant argument is supported",
            ),
        }
    } else {
        JsValue::Unknown(
            Some(Arc::new(JsValue::call(
                box JsValue::WellKnownFunction(WellKnownFunctionKind::PathToFileUrl),
                args,
            ))),
            "only a single argument is supported",
        )
    }
}

pub fn well_known_function_member(kind: WellKnownFunctionKind, prop: JsValue) -> JsValue {
    match (&kind, prop.as_str()) {
        (WellKnownFunctionKind::Require, Some("resolve")) => {
            JsValue::WellKnownFunction(WellKnownFunctionKind::RequireResolve)
        }
        _ => JsValue::Unknown(
            Some(Arc::new(JsValue::member(
                box JsValue::WellKnownFunction(kind),
                box prop,
            ))),
            "unsupported property on function",
        ),
    }
}

pub fn well_known_object_member(kind: WellKnownObjectKind, prop: JsValue) -> JsValue {
    match kind {
        WellKnownObjectKind::PathModule => path_module_member(prop),
        WellKnownObjectKind::FsModule => fs_module_member(prop),
        WellKnownObjectKind::UrlModule => url_module_member(prop),
        WellKnownObjectKind::ChildProcess => child_process_module_member(prop),
        #[allow(unreachable_patterns)]
        _ => JsValue::Unknown(
            Some(Arc::new(JsValue::member(
                box JsValue::WellKnownObject(kind),
                box prop,
            ))),
            "unsupported object kind",
        ),
    }
}

pub fn path_module_member(prop: JsValue) -> JsValue {
    match prop.as_str() {
        Some("join") => JsValue::WellKnownFunction(WellKnownFunctionKind::PathJoin),
        Some("dirname") => JsValue::WellKnownFunction(WellKnownFunctionKind::PathDirname),
        _ => JsValue::Unknown(
            Some(Arc::new(JsValue::member(
                box JsValue::WellKnownObject(WellKnownObjectKind::PathModule),
                box prop,
            ))),
            "unsupported property on Node.js path module",
        ),
    }
}

pub fn fs_module_member(prop: JsValue) -> JsValue {
    if let Some(word) = prop.as_word() {
        match &**word {
            "realpath" | "realpathSync" | "stat" | "statSync" | "existsSync"
            | "createReadStream" | "exists" | "open" | "openSync" | "readFile" | "readFileSync" => {
                return JsValue::WellKnownFunction(WellKnownFunctionKind::FsReadMethod(
                    word.clone(),
                ))
            }
            "promises" => return JsValue::WellKnownObject(WellKnownObjectKind::FsModule),
            _ => {}
        }
    }
    JsValue::Unknown(
        Some(Arc::new(JsValue::member(
            box JsValue::WellKnownObject(WellKnownObjectKind::FsModule),
            box prop,
        ))),
        "unsupported property on Node.js fs module",
    )
}

pub fn url_module_member(prop: JsValue) -> JsValue {
    match prop.as_str() {
        Some("pathToFileURL") => JsValue::WellKnownFunction(WellKnownFunctionKind::PathToFileUrl),
        _ => JsValue::Unknown(
            Some(Arc::new(JsValue::member(
                box JsValue::WellKnownObject(WellKnownObjectKind::UrlModule),
                box prop,
            ))),
            "unsupported property on Node.js url module",
        ),
    }
}

pub fn child_process_module_member(prop: JsValue) -> JsValue {
    match prop.as_str() {
        Some("spawn") | Some("spawnSync") | Some("execFile") | Some("execFileSync") => {
            JsValue::WellKnownFunction(WellKnownFunctionKind::ChildProcessSpawnMethod(
                prop.as_word().unwrap().clone(),
            ))
        }
        Some("fork") => JsValue::WellKnownFunction(WellKnownFunctionKind::ChildProcessFork),
        _ => JsValue::Unknown(
            Some(Arc::new(JsValue::member(
                box JsValue::WellKnownObject(WellKnownObjectKind::ChildProcess),
                box prop,
            ))),
            "unsupported property on Node.js child_process module",
        ),
    }
}