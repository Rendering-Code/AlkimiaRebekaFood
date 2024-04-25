use std::{collections::HashMap, sync::Mutex};
use reqwest::{header::USER_AGENT, Client, Error};
use teloxide::{prelude::*, types::{MessageId, MessageKind}, utils::command::BotCommands};
use html2text::from_read;
use once_cell::sync::Lazy;

struct Menu
{
    entrants: Vec<String>,
    seconds: Vec<String>,
}

struct PollIds
{
    entrants_id: MessageId,
    seconds_id: MessageId
}

static mut LAST_POLLS: Lazy<Mutex<HashMap::<ChatId, Option<PollIds>>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();
    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Lista de comandos:")]
enum Command {
    #[command(description = "Comando de ayuda")]
    Help,
    #[command(description = "Crear la poll para entrantes y segundos.")]
    MakePoll,
    #[command(description = "Decide quien de los que hayan votado llama hoy.")]
    WhoCalls,
    TestPolls
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::MakePoll => 
        {
            let option_menu = get_menu().await.unwrap_or_else(|_| None);
            if let Some(menu) = option_menu
            {
                let mut entrants_poll = bot.send_poll(msg.chat.id, String::from("Entrantes"), menu.entrants);
                entrants_poll.allows_multiple_answers = Some(true);
                entrants_poll.is_anonymous = Some(false);
                entrants_poll.disable_notification = Some(false);
                let first_poll_message = entrants_poll.send().await?;
                
                let mut seconds_poll = bot.send_poll(msg.chat.id, String::from("Segundos"), menu.seconds);
                seconds_poll.allows_multiple_answers = Some(true);
                seconds_poll.is_anonymous = Some(false);
                seconds_poll.disable_notification = Some(false);
                let second_poll_message = seconds_poll.send().await?;

                unsafe
                {
                    let mut last_poll_guard = LAST_POLLS.lock().unwrap();
                    last_poll_guard.insert(msg.chat.id.clone(), Some(PollIds{entrants_id: first_poll_message.id, seconds_id: second_poll_message.id}));
                }

                second_poll_message
            }
            else 
            {
                bot.send_message(msg.chat.id, "El menu esta algo raro hoy, no he podido pasarlo a poll. Quejas a Roger!!").await?
            }
        },
        Command::WhoCalls =>
        {
            let mut name = "No se quien eres".to_string();
            if let MessageKind::Common(message) = msg.kind
            {
                if let Some(user) = message.from
                {
                    name = user.first_name;
                }
            }
            bot.send_message(msg.chat.id, format!("{}, deja de preguntar, aun no se sabe!", name)).await?
        }
        Command::TestPolls => 
        {
            bot.send_message(msg.chat.id, "WIP").await?
        }
    };

    Ok(())
}

async fn get_menu() -> Result<Option<Menu>, Error>
{
    let client = Client::new();
    let result = client.get("http://restauranterebeka.com/menu/")
        .header(USER_AGENT, "AlkimiaBot/0.1")
        .send()
        .await?
        .text()
        .await?;

    let binding = from_read(result.as_bytes(), 200);
    let menu: Vec<&str> = binding.split("\n").filter(|x| !x.is_empty()).collect();
    let entrants_index = menu.iter().position(|&x| x.contains("**ENTRANTES**")).unwrap();
    let end_index = menu.iter().position(|&x| x.contains("**Menú completo**")).unwrap();
    let real_menu = &menu[entrants_index..end_index];
    let second_plates = real_menu.iter().position(|&x| x.contains("**SEGUNDOS**")).unwrap();
    let mut all_entrants: Vec<String> = real_menu[..second_plates]
        .iter()
        .filter(|&x| !x.contains("*"))
        .map(|&x| x[..25].to_lowercase().to_string())
        .collect();
    all_entrants.iter_mut().for_each(|x| x.push_str("..."));

    let mut all_seconds: Vec<String> = real_menu[second_plates..]
        .iter()
        .filter(|&x| !x.contains("*"))
        .map(|&x| x[..25].to_lowercase().to_string())
        .collect();
    all_seconds.iter_mut().for_each(|x| x.push_str("..."));
    Ok(Some(Menu{entrants: all_entrants, seconds: all_seconds}))
}
