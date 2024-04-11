use serde::Serialize;
use tokio::runtime::Runtime;

use super::super::Notifiable;
use std::error::Error;

///https://open.dingtalk.com/document/orgapp/custom-robot-access
pub struct DingTalk {
    webhook: String,
    template: String,
}
#[derive(Debug, Serialize)]
struct Content {
    content: String,
}
#[derive(Debug, Serialize)]
struct Body {
    msgtype: String,
    text: Content,
}

impl DingTalk {
    pub fn new(webhook: String, template: String) -> Self {
        Self { webhook, template }
    }

    async fn send(&self, message: &str) -> bool {
        let client = reqwest::Client::new();
        let body = Body {
            msgtype: "text".to_owned(),
            text: Content {
                content: self.template.replace("{message}", message),
            },
        };
        let response = client.post(&self.webhook).json(&body).send().await;
        match response {
            Ok(r) => {
                return r.status().as_u16() == 200;
            }
            Err(e) => {
                log::error!("DingTalk notify error: {}", e);
            }
        }
        false
    }
}

impl Notifiable for DingTalk {
    fn notify(&self, message: &str) -> Result<bool, Box<dyn Error>> {
        let future = self.send(message);
        Ok(Runtime::new().unwrap().block_on(future))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dingtalk() {
        let dingtalk =
            DingTalk::new("https://oapi.dingtalk.com/robot/send?access_token=XXXXXXXXXXXXXXXXXXXX".to_owned(),
            "Notice: {message}".to_owned());
        let ret = dingtalk.notify("Hello, World!");
        assert!(ret.is_ok());
    }
}
