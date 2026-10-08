use clap::Parser;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "rest-reminder-gen", about = "用阿里云万相生成 Rest Reminder Pet 形象包")]
struct Args {
    #[arg(long)]
    name: String,
    #[arg(long)]
    prompt: String,
    #[arg(long)]
    api_key: Option<String>,
    #[arg(long)]
    endpoint: Option<String>,
}

const MODEL: &str = "wan2.6-t2i";
const SUFFIX: &str =
    "cute original cartoon cat mascot, transparent background, consistent character, not a copyrighted character, simple chibi style";

const ACTIONS: [(&str, &str, &[&str]); 4] = [
    (
        "crawl",
        "climbing down from the top edge of a screen, side view, paws gripping",
        &["crawl_01.png", "crawl_02.png", "crawl_03.png"],
    ),
    (
        "lookDown",
        "hanging from the top, looking downward curiously",
        &["look_01.png", "look_02.png"],
    ),
    (
        "happyClimb",
        "happily climbing back up, cheerful expression",
        &["climb_01.png", "climb_02.png"],
    ),
    (
        "fall",
        "falling down with flailing paws, cartoonish",
        &["fall_01.png", "fall_02.png", "fall_03.png"],
    ),
];

fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rest-reminder-pet")
}

fn load_key_from_config() -> Option<String> {
    let path = app_data_dir().join("config.json");
    let text = fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(&text).ok()?;
    v.get("wanxiang")
        .and_then(|w| w.get("apiKey"))
        .and_then(|k| k.as_str())
        .map(|s| s.to_string())
}

fn main() {
    let args = Args::parse();
    let api_key = args
        .api_key
        .or_else(|| std::env::var("DASHSCOPE_API_KEY").ok())
        .or_else(load_key_from_config);

    let Some(api_key) = api_key.filter(|s| !s.is_empty()) else {
        eprintln!("错误: 未找到 API Key。请设置 DASHSCOPE_API_KEY、传入 --api-key，或在设置中保存 wanxiang.apiKey。");
        std::process::exit(1);
    };

    let endpoint = args.endpoint.unwrap_or_else(|| {
        "https://dashscope.aliyuncs.com/api/v1/services/aigc/text2image/image-synthesis".into()
    });

    let safe: String = args
        .name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let out_dir = app_data_dir().join("characters").join(&safe);
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("错误: 无法创建目录 {}: {e}", out_dir.display());
        std::process::exit(1);
    }

    let client = reqwest::blocking::Client::new();
    let mut actions_json = serde_json::Map::new();
    let mut failed = false;

    for (action, pose, frames) in ACTIONS {
        let mut frame_names = Vec::new();
        for (i, file) in frames.iter().enumerate() {
            let prompt = format!(
                "{}, {}, animation frame {}, {}, {}",
                args.prompt, pose, i + 1, SUFFIX, "square composition centered"
            );
            eprint!("生成 {action}/{file} ... ");
            match generate_one(&client, &endpoint, &api_key, &prompt, &out_dir.join(file)) {
                Ok(()) => {
                    eprintln!("OK");
                    frame_names.push((*file).to_string());
                }
                Err(e) => {
                    eprintln!("失败: {e}");
                    failed = true;
                }
            }
            thread::sleep(Duration::from_millis(400));
        }
        if !frame_names.is_empty() {
            actions_json.insert(
                action.to_string(),
                json!({
                    "fps": match action {
                        "lookDown" => 4,
                        "fall" => 10,
                        _ => 8,
                    },
                    "frames": frame_names,
                }),
            );
        }
    }

    let manifest = json!({
        "name": safe,
        "frameSize": { "w": 128, "h": 128 },
        "actions": actions_json,
    });
    if let Err(e) = fs::write(
        out_dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap(),
    ) {
        eprintln!("错误: 写入 manifest 失败: {e}");
        std::process::exit(1);
    }

    if failed {
        eprintln!(
            "部分帧失败，已写入可用帧到 {}。可重跑命令补全。",
            out_dir.display()
        );
        std::process::exit(2);
    }

    println!("形象包已生成: {}", out_dir.display());
}

fn generate_one(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    api_key: &str,
    prompt: &str,
    out_file: &Path,
) -> Result<(), String> {
    let body = json!({
        "model": MODEL,
        "input": { "prompt": prompt },
        "parameters": {
            "size": "1024*1024",
            "n": 1
        }
    });

    let resp = client
        .post(endpoint)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("X-DashScope-Async", "enable")
        .json(&body)
        .send()
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let v: Value = resp.json().map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}: {v}"));
    }

    // Sync-style: output may contain url directly; async: task id
    if let Some(url) = v
        .pointer("/output/results/0/url")
        .or_else(|| v.pointer("/output/choices/0/message/content/0/image"))
        .and_then(|x| x.as_str())
    {
        return download(client, url, out_file);
    }

    let task_id = v
        .pointer("/output/task_id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| format!("无法解析响应: {v}"))?;

    let task_url = format!("https://dashscope.aliyuncs.com/api/v1/tasks/{task_id}");
    for _ in 0..60 {
        thread::sleep(Duration::from_secs(2));
        let task_resp = client
            .get(&task_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .send()
            .map_err(|e| e.to_string())?;
        let tv: Value = task_resp.json().map_err(|e| e.to_string())?;
        let state = tv
            .pointer("/output/task_status")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        if state == "SUCCEEDED" {
            let url = tv
                .pointer("/output/results/0/url")
                .and_then(|x| x.as_str())
                .ok_or_else(|| format!("任务成功但无图片 URL: {tv}"))?;
            return download(client, url, out_file);
        }
        if state == "FAILED" || state == "UNKNOWN" {
            return Err(format!("任务失败: {tv}"));
        }
    }
    Err("任务超时".into())
}

fn download(client: &reqwest::blocking::Client, url: &str, out_file: &Path) -> Result<(), String> {
    let bytes = client
        .get(url)
        .send()
        .map_err(|e| e.to_string())?
        .bytes()
        .map_err(|e| e.to_string())?;
    fs::write(out_file, bytes).map_err(|e| e.to_string())
}
