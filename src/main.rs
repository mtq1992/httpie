use clap::Parser;
use colored::Colorize;
use mime::Mime;
use reqwest::{header, Client, Response, Url};
use anyhow::{anyhow, Result};
use std::{collections::HashMap, str::FromStr};
use syntect::{
    easy::HighlightLines, 
    parsing::SyntaxSet, 
    highlighting::{Style, ThemeSet},
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};



/// A naive httpie implemention with Rust, can you imagine how easy it is?

#[derive(Parser, Debug)]
#[clap(version = "1.0", author = "Tianqi Ma <mtqmx3@gmail.com>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

// get/post
#[derive(Parser, Debug)]
enum SubCommand {
    Get(Get),
    Post(Post),
}

// get 子命令

/// feed get with an url and we will retrieve the response for you
#[derive(Parser, Debug)]
struct Get{
    // HTTP 请求的 url
    #[clap(parse(try_from_str = parse_url))]
    url: String,
}

// post 子命令

/// feed post with an url and optional key=value pairs. We will post the data 
/// as JSON, and retrieve the response for you.
#[derive(Parser, Debug)]
struct Post{
    // HTTP 请求的 url
    #[clap(parse(try_from_str = parse_url))]
    url: String,
    // HTTP 请求的 body
    #[clap(parse(try_from_str = parse_kv_pair))]
    body: Vec<KvPair>,
}

/// 命令行中的 k=v 使用 parse_kv_pair 解析成 KvPair struct
#[derive(Debug)]
struct KvPair {
    k: String,
    v: String,
}

// 实现 FromStr trait 
impl FromStr for KvPair {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split('=');
        let err = || anyhow!(format!("Failed to parse {}", s));
        Ok(Self {
            k: (split.next().ok_or_else(err)?).to_string(),
            v: (split.next().ok_or_else(err)?).to_string(),
        })
    }
}

fn parse_kv_pair(s: &str) -> Result<KvPair> {
    s.parse()
}

fn parse_url(s: &str) -> Result<String> {
    // 使用 reqwest::Url 检查 url 是否合法
    let _url: Url = s.parse()?;

    Ok(s.into())
}

async fn get(client: Client, args:&Get) -> Result<()> {
    let resp = client.get(&args.url).send().await?;
    
    Ok(print_resp(resp).await?)
}

async fn post(client: Client,args: &Post) -> Result<()> {
    let mut body = HashMap::new();
    for pair in args.body.iter() {
        body.insert(&pair.k, &pair.v);
    }

    let resp = client.post(&args.url).json(&body).send().await?;
    println!("{:?}", resp.text().await?);
    Ok(())
}

// 打印服务器版本号和状态码
fn print_status(resp: &Response) {
    let status = format!("{:?} {}", resp.version(), resp.status()).blue();
    println!("{}\n", status);
}

// 打印服务器返回的 http header
fn print_headers(resp: &Response) {
    for (name, value) in resp.headers() {
        println!("{}: {:?}", name.to_string().green(), value);
    }
    println!()
}

// 打印服务器返回的 http body
fn print_body(m: Option<Mime>, body: &str) {
    match m {
        Some(v) if v == mime::APPLICATION_JSON => print_syntect(body, "json"),
        Some(v) if v == mime::TEXT_HTML => print_syntect(body, "html"),
        _ => println!("{}", body),
    }
}

async fn print_resp(resp: Response) -> Result<()> {
    print_status(&resp);
    print_headers(&resp);
    let mime = get_content_type(&resp);
    let body = resp.text().await?;
    print_body(mime, &body);
    Ok(())
}

fn get_content_type(resp: &Response) -> Option<Mime> {
    resp.headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().parse().unwrap())

}


#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let mut headers = header::HeaderMap::new();

    headers.insert("X-POWERED-BY", "Rust".parse()?);
    headers.insert(header::USER_AGENT, "Rust Httpie".parse()?);
    // 生成一个 http 客户端
    let client = reqwest::Client::builder().default_headers(headers).build()?;
    let result = match opts.subcmd {
        SubCommand::Get(ref args) => get(client, args).await?, 
        SubCommand::Post(ref args) => post(client, args).await?,
    };

    Ok(result)
}

fn print_syntect(s: &str, ext: &str) {
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let syntax = ps.find_syntax_by_extension(ext).unwrap();
    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    for line in LinesWithEndings::from(s) {
        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true);
        println!("{}", escaped);
    }
}

