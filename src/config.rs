use super::chat::record::Channel;
use regex::Regex;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml;
#[derive(Debug, Deserialize, Clone)]
pub struct Game {
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Simple {}

#[derive(Debug, Deserialize, Clone)]
pub struct Console {
    pub color: String,
    pub format: String,
    pub by_log: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Ringtone {
    pub audio: String,
    pub device: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Dingtalk {
    pub webhook: String,
    pub template: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Invoke {
    pub path: String,
    pub workdir: String,
    pub args: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Notifier {
    pub simple: Simple,
    pub console: Console,
    pub ringtone: Ringtone,
    pub dingtalk: Dingtalk,
    pub invoke: Invoke,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Trigger {
    pub regex: String,
    pub format: String,
    pub channel: String,
    pub notifier: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub game: Game,
    pub notifier: Notifier,
    pub trigger: Vec<Trigger>,
}

impl Notifier {
    pub fn find(
        cfg: &Config,
        name: &str,
    ) -> Result<Box<dyn super::Notifiable>, Box<dyn std::error::Error>> {
        match name {
            "simple" => Ok(Box::new(super::notifier::Simple::new())),
            "console" => {
                let cc = &cfg.notifier.console;
                Ok(Box::new(super::notifier::Console::new(
                    cc.color.clone(),
                    cc.format.clone(),
                    cc.by_log,
                )))
            }
            "ringtone" => {
                let rc = &cfg.notifier.ringtone;
                let o = super::notifier::Ringtone::new(rc.audio.clone(), rc.device.clone())?;
                Ok(Box::new(o))
            }
            "dingtalk" => {
                let dc = &cfg.notifier.dingtalk;
                Ok(Box::new(super::notifier::webhook::DingTalk::new(
                    dc.webhook.clone(),
                    dc.template.clone(),
                )))
            }
            "invoke" => {
                let ic = &cfg.notifier.invoke;
                Ok(Box::new(super::notifier::Invoke::new(
                    ic.path.clone(),
                    ic.args.clone(),
                    ic.workdir.clone(),
                )))
            }
            _ => Err(format!("Not found notifier {name}").into()),
        }
    }
}

impl Trigger {
    #[allow(dead_code)]
    fn new(regex: &str) -> Self {
        Self {
            regex: regex.to_owned(),
            format: String::new(),
            channel: String::new(),
            notifier: Vec::new(),
        }
    }

    pub fn try_match(&self, text: &str) -> Option<Vec<String>> {
        let mut matched = Vec::new();
        let re = Regex::new(&self.regex).unwrap();
        if let Some(caps) = re.captures(text) {
            for i in 0..caps.len() {
                if let Some(m) = caps.get(i) {
                    matched.push(m.as_str().to_owned());
                }
            }
            Some(matched)
        } else {
            None
        }
    }

    pub fn format(&self, matched: &Vec<String>) -> String {
        let mut fmt = self.format.clone();
        for (i, m) in matched.iter().enumerate() {
            fmt = fmt.replace(&format!("{{{}}}", i), m);
        }
        fmt
    }

    pub fn accept(&self, channel: &Channel) -> bool {
        match self.channel.to_lowercase().as_str() {
            "world" => channel == &Channel::World,
            "group" => channel == &Channel::Group,
            "region" => channel == &Channel::Region,
            "common" => channel == &Channel::Common,
            _ => true,
        }
    }
}

impl Config {
    pub fn parse(text: &mut String) -> Result<Self, toml::de::Error> {
        toml::from_str(&text)
    }
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = File::open(path)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        Self::parse(&mut text).map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let path = "config.toml";
        let config = Config::load(path).unwrap();
        println!("{:?}", config);
    }

    #[test]
    fn test_notify_try_match() {
        let text = r#"你感觉到一股不可思议的力量，而『挑战赛通道』好像快消失了。"#;
        let matched = Trigger::new(r#"你感觉到一股不可思议的力量，而『(\w+)』好像快消失了。"#)
            .try_match(text)
            .unwrap();
        println!("{:?}", matched);
        assert!(matched.len() == 2);

        let matched = Trigger::new(r#"你感觉到一股不可思议的力量"#)
            .try_match(text)
            .unwrap();
        println!("{:?}", matched);
        assert!(matched.len() == 1);

        let matched = Trigger::new(r#"丂你感觉到一股不可思议的力量"#).try_match(text);
        println!("{:?}", matched);
        assert!(matched.is_none());
    }

    #[test]
    fn test_notify_match_group() {
        let text = r#"画眉鸟离开了队伍。"#;
        let trigger = Trigger::new(r#"(\w+)离开了队伍。"#);
        let matched = trigger.try_match(text).unwrap();
        println!("{:?}", matched);
        assert!(matched[1] == "画眉鸟");
    }
    #[test]
    fn test_notify_match_card() {
        let text = r#"您账号剩余时间为1小时1分26秒"#;
        let trigger = Trigger::new(r#"您账号剩余时间为(\w+)"#);
        let matched = trigger.try_match(text).unwrap();
        println!("{:?}", matched);
        assert!(matched[1] == "1小时1分26秒");
    }

    #[test]
    fn test_trigger_format() {
        let text = r#"你感觉到一股不可思议的力量，而『挑战赛通道』好像快消失了。"#;
        let trigger = Trigger::new(r#"你感觉到一股不可思议的力量，而『(\w+)』好像快消失了。"#);
        let matched = trigger.try_match(text).unwrap();
        println!("{:?}", matched);
        let fmt = trigger.format(&matched);
        println!("{:?}", fmt);
    }
}
