use std::{
    fs,
    path::Path,
    sync::{Arc, OnceLock},
};
mod ws_popup;
use aviutl2::{anyhow, generic::GenericPlugin, ldbg, log};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};
use serde::{Deserialize, Serialize};
use tap::Pipe;
use ws_popup::WsPopup;
mod parser;
use parser::parse_candicates;

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    name: String,
    label: String,
}

#[derive(Deserialize)]
struct SearchQuery {
    query: String,
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
    _app_thread: std::thread::JoinHandle<()>,
    app_flag: std::sync::mpsc::Sender<()>,
    candicates: Arc<Vec<Candidate>>,
}

unsafe impl Send for ObjectSearchPlugin {}
unsafe impl Sync for ObjectSearchPlugin {}

static WEB_CONTENT: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/page/dist");

impl GenericPlugin for ObjectSearchPlugin {
    fn new(info: aviutl2::AviUtl2Info) -> aviutl2::AnyResult<Self> {
        let mut current_dir = std::env::current_exe()?;
        current_dir.pop();

        let mut candicates = Vec::new();
        'exist_check: {
            let path1 = current_dir.join("data/aviutl2.ini");
            if path1.exists() {
                let content = fs::read_to_string(&path1)?;
                candicates = parse_candicates(&content).unwrap_or_default();
                break 'exist_check;
            }

            let path2 = current_dir.join("aviutl2.ini");
            if path2.exists() {
                let content = fs::read_to_string(&path2)?;
                candicates = parse_candicates(&content).unwrap_or_default();
                break 'exist_check;
            }

            let path = Path::new("C:/ProgramData/aviutl2/aviutl2.ini");
            if path.exists() {
                let content = fs::read_to_string(Path::new("C:/ProgramData/aviutl2.ini"))?;
                candicates = parse_candicates(&content).unwrap_or_default();
                break 'exist_check;
            }
        }

        let candicates = Arc::new(candicates);
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

        let (sender, receiver) = std::sync::mpsc::channel();
        let app_thread = Self::spawn_app_thread(Arc::clone(&edit_handle), receiver);

        Ok(Self {
            webview,
            window,
            edit_handle,

            _app_thread: app_thread,
            app_flag: sender,
            candicates,
        })
    }

    fn register(&mut self, registry: &mut aviutl2::generic::HostAppHandle) {
        let handle = registry.create_edit_handle();
        let _ = self.edit_handle.set(handle);

        registry
            .register_window_client("Object Search", &self.window)
            .unwrap();
    }
}

impl ObjectSearchPlugin {
    fn spawn_app_thread(
        edit_handle: Arc<OnceLock<aviutl2::generic::EditHandle>>,
        replace_flag_rx: std::sync::mpsc::Receiver<()>,
    ) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            loop {
                // Wait for a replace signal
                if replace_flag_rx.recv().is_err() {
                    break;
                }

                let _ = edit_handle
                    .wait()
                    .call_edit_section(move |section| anyhow::Ok(()));
            }
        })
    }

    fn ipc_handler(message_str: String) {
        if let Ok(query_data) = serde_json::from_str::<SearchQuery>(&message_str) {
            let query = query_data.query.trim().to_lowercase();

            Self::with_instance(|instance| {
                instance.search_query(&query);
            })
        }
    }

    fn search_query(&self, query: &str) {
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
        let json_response =
            serde_json::to_string(&response).unwrap_or_else(|_| r#"{"candidates":[]}"#.to_string());

        // 5. JavaScriptのコールバック関数を呼び出して結果を渡す
        let script = format!("receive_search_results({})", json_response);
        let _ = self.webview.evaluate_script(&script);
    }
}

aviutl2::register_generic_plugin!(ObjectSearchPlugin);
