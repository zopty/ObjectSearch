use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
    sync::{Arc, OnceLock},
};
mod ws_popup;
use aviutl2::{alias::Table, generic::GenericPlugin, ldbg, log};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::Serialize;
use tap::Pipe;
use ws_popup::WsPopup;

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    name: String,
    label: String,
}

#[derive(Serialize)]
struct SearchResult {
    candidates: Vec<Candidate>,
}

#[aviutl2::plugin(GenericPlugin)]
struct ObjectSearchPlugin {
    webview: wry::WebView,
    window: WsPopup,

    edit_handle: Arc<OnceLock<aviutl2::generic::EditHandle>>,
    candicates: Arc<Vec<Candidate>>,
}

unsafe impl Send for ObjectSearchPlugin {}
unsafe impl Sync for ObjectSearchPlugin {}

static WEB_CONTENT: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR\\page\\dist");

impl GenericPlugin for ObjectSearchPlugin {
    fn new(_info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        let mut current_dir = std::env::current_exe()?;
        current_dir.pop();

        let mut ini_path = Path::new("");
        'exist_check: {
            let path1 = current_dir.join("data\\aviutl2.ini");
            if path1.exists() {
                ini_path = Box::leak(Box::new(path1));
                break 'exist_check;
            }

            let path2 = current_dir.join("aviutl2.ini");
            if path2.exists() {
                ini_path = Box::leak(Box::new(path2));
                break 'exist_check;
            }

            let path3 = Path::new("C:\\ProgramData\\aviutl2\\aviutl2.ini");
            if path3.exists() {
                ini_path = Box::leak(Box::new(path3));
                break 'exist_check;
            }
            ldbg!("cannot find ini file");
        }

        let file = File::open(ini_path).expect("cannot open ini file");
        let reader = BufReader::new(file);

        let mut candidates = Vec::new();
        let mut current_effect_name: Option<String> = None;
        for line_result in reader.lines() {
            match line_result {
                Ok(line) => {
                    if line.starts_with("[Effect.") {
                        current_effect_name = Some(
                            line.trim_start_matches("[Effect.")
                                .trim_end_matches("]")
                                .to_string(),
                        );
                        continue;
                    }
                    if line.starts_with("label=") && current_effect_name.is_some() {
                        let name = current_effect_name.take().unwrap();
                        let label = line.trim_start_matches("label=").to_string();
                        if !label.is_empty() {
                            candidates.push(Candidate { name, label });
                            current_effect_name = None;
                        }
                        continue;
                    }
                }
                Err(error) => log::error!("{}", error),
            }
        }

        let candicates = Arc::new(candidates);
        let edit_handle = Arc::new(OnceLock::<aviutl2::generic::EditHandle>::new());

        let window = WsPopup::new("Object Search", (800, 600))?;
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("object-search-plugin");
        let mut web_context = wry::WebContext::new(Some(cache_dir));
        let webview = wry::WebViewBuilder::new_with_web_context(&mut web_context)
            .with_ipc_handler(|payload| {
                let message_str = payload.into_body();
                Self::ipc_handler(message_str);
            })
            .pipe(|builder| {
                if cfg!(debug_assertions) {
                    log::info!("Running in development mode, loading from localhost:5173");
                    builder.with_url("http://localhost:5173")
                } else {
                    builder
                        .with_custom_protocol("app".to_string(), move |_id, request| {
                            let mut path = request.uri().path().trim_start_matches("/");
                            if path == "" {
                                path = "index.html";
                            }
                            if let Some(file) = WEB_CONTENT.get_file(path) {
                                let mime = mime_guess::from_path(path).first_or_octet_stream();
                                wry::http::Response::builder()
                                    .header("Content-Type", mime.as_ref())
                                    .body(file.contents().to_vec().into())
                                    .unwrap()
                            } else {
                                wry::http::Response::builder()
                                    .status(404)
                                    .body(Vec::new().into())
                                    .unwrap()
                            }
                        })
                        .with_url("app://index.html")
                }
            })
            .build(&window)?;

        Ok(Self {
            webview,
            window,
            edit_handle,

            candicates,
        })
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        registry.set_plugin_information("Object Search");

        let handle = registry.create_edit_handle();
        let _ = self.edit_handle.set(handle);

        registry
            .register_window_client("Object Search", &self.window)
            .unwrap();
    }
}

impl ObjectSearchPlugin {
    fn send_to_webview<T: serde::Serialize>(&self, name: &str, data: &T) {
        log::debug!("Sending to webview: {}", name);
        match serde_json::to_value(data) {
            Ok(json) => {
                let json = serde_json::json!({ "type": name, "data": json }).to_string();
                let script = format!(
                    "try {{ window.bridge && window.bridge._emit({json}); }} catch(e) {{ console.error(e); }}"
                );
                let _ = self.webview.evaluate_script(&script);
                log::debug!("Sent to webview: {}", name);
            }
            Err(e) => {
                log::error!("Failed to serialize data for webview: {}", e);
            }
        }
    }

    fn ipc_handler(message_str: String) {
        #[derive(serde::Deserialize, Debug)]
        #[serde(tag = "type", content = "data", rename_all = "snake_case")]
        enum IpcMessage {
            Search(String),
            Select(String),
        }

        match serde_json::from_str::<IpcMessage>(&message_str) {
            Ok(msg) => {
                log::debug!("IPC message received: {:?}", msg);
                match msg {
                    IpcMessage::Search(query) => {
                        let query = query.trim().to_lowercase();

                        Self::with_instance(|instance| {
                            let result = instance.search_query(&query);
                            instance.send_to_webview("search_result", &result);
                        })
                    }
                    IpcMessage::Select(name) => Self::with_instance(|instance| {
                        instance
                            .edit_handle
                            .wait()
                            .call_edit_section(|section| {
                                let mut layer = 0;
                                let mut start = 0;
                                let mut end = 60;

                                if let Some(obj) = section
                                    .get_focused_object()
                                    .expect("unable to get focused object")
                                {
                                    let object = section.object(&obj);
                                    let layer_frame = object
                                        .get_layer_frame()
                                        .expect("unable to get layer frame");
                                    layer = layer_frame.layer;
                                    start = layer_frame.start;
                                    end = layer_frame.end;
                                }

                                let mut root = Table::new();
                                root.insert_value("frame", format!("{},{}", start, end));

                                let mut effect = Table::new();
                                effect.insert_value("effect.name", &name);

                                let mut table = Table::new();
                                table.insert_table("Object", root);
                                table.insert_table("Object.0", effect);

                                loop {
                                    match section.create_object_from_alias(
                                        &table.to_string(),
                                        layer,
                                        start,
                                        end - start,
                                    ) {
                                        Ok(_) => {
                                            break;
                                        }
                                        Err(_) => {
                                            layer += 1;
                                            if layer >= 10 {
                                                log::info!("too many retries, stopped insertion");
                                                break;
                                            }
                                        }
                                    };
                                }
                            })
                            .expect("unable to call edit section");
                    }),
                }
            }
            Err(error) => {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&message_str) {
                    if let Some(ty) = value.get("type").and_then(|v| v.as_str()) {
                        match ty {
                            "search_query" => {
                                log::error!(
                                    "Failed to search query from IPC message data: {:?}",
                                    value.get("data")
                                );
                            }
                            other => {
                                log::warn!("Unknown IPC message type: {}", other);
                            }
                        }
                    } else {
                        log::error!("Failed to parse IPC message: {}", error);
                    }
                } else {
                    log::error!("Failed to parse IPC message: {}", error);
                }
            }
        }
    }

    fn search_query(&self, query: &str) -> SearchResult {
        let matcher = SkimMatcherV2::default();
        let mut filtered_candidates = Vec::new();

        let candidates_for_matcher = self.candicates.clone();

        if !query.is_empty() {
            for cand in candidates_for_matcher.iter() {
                // 検索対象文字列を生成 (例: "Fire Effect Alpha (effect.Fire_A)")
                let target = format!("{} ({})", cand.label, cand.name);

                // ファジーマッチングを実行し、スコアを取得
                if let Some(score) = matcher.fuzzy_match(&target, &query) {
                    // スコアが一定以上の場合に候補として追加
                    filtered_candidates.push((score, cand.clone()));
                }
            }

            // スコアの高い順にソート
            filtered_candidates.sort_by(|a, b| b.0.cmp(&a.0));
        } else {
            // クエリが空の場合は全候補を表示
            filtered_candidates = candidates_for_matcher
                .iter()
                .map(|c| (0, c.clone()))
                .collect();
        }

        // 4. 結果のJSONをJavaScriptに返す
        let results = filtered_candidates
            .into_iter()
            .map(|(_, cand)| cand)
            .collect();

        let response = SearchResult {
            candidates: results,
        };

        response
    }
}

aviutl2::register_generic_plugin!(ObjectSearchPlugin);
