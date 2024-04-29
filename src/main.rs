use std::{collections::HashMap, sync::{Arc, Mutex}};
use reqwest::{header::USER_AGENT, Client, Error};
use teloxide::{dispatching::UpdateFilterExt, prelude::*, update_listeners, utils::command::BotCommands, RequestError};
use html2text::from_read;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};

struct Menu
{
    entrants: Vec<String>,
    seconds: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct BotProgress
{
    chats_data: HashMap<i64, Vec<PlayerScore>>,
}

#[derive(Serialize, Deserialize)]
struct PlayerScore
{
    polls_made: u16,
    calls_made: u16,
    xl_salads: u16,
    fastest_answering: u16,
    slowest_answering: u16,
    retracted_votes: u16,
}

struct RebekaPollData
{
    entrants_id: String,
    seconds_id: String,
    participants: Vec<String>
}

static mut LAST_POLLS: Lazy<Mutex<HashMap::<ChatId, RebekaPollData>>> = Lazy::new(|| Mutex::new(HashMap::new()));

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();
    let def_handle = |_upd: Arc::<Update>| Box::pin(async {});
    
    Dispatcher::builder(
        bot.clone(),
        dptree::entry()
            .branch(Update::filter_message().filter_command::<Command>().endpoint(answer))
            .branch(Update::filter_poll_answer().endpoint(answer_poll)),
        )
        .default_handler(def_handle)
        .enable_ctrlc_handler()
        .build()
        .dispatch_with_listener(
            update_listeners::polling_default(bot.clone()).await,
            LoggingErrorHandler::with_custom_text("An error from the update listener"),
        )
        .await;
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
    #[command(description = "Lista de ranking de polls creadas")]
    RankPolls,
    #[command(description = "Lista de ranking llamadas hechas")]
    RankCalls,
    #[command(description = "Lista de ranking de ensaladas XL pedidas")]
    RankSaladsXL,
    #[command(description = "Lista de ranking del que ha pedido mas rapido")]
    RankFastest,
    #[command(description = "Lista de ranking del que ha pedido mas lento")]
    RankSlowest,
    #[command(description = "Lista de ranking del que ha cambiado mas su voto")]
    RankRetracts,

}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::MakePoll => make_poll(&bot, &msg).await?,
        Command::WhoCalls => who_calls(&bot, &msg).await?,
        Command::RankPolls => wip(&bot, &msg).await?,
        Command::RankCalls => wip(&bot, &msg).await?,
        Command::RankSaladsXL => wip(&bot, &msg).await?,
        Command::RankFastest => wip(&bot, &msg).await?,
        Command::RankSlowest => wip(&bot, &msg).await?,
        Command::RankRetracts => wip(&bot, &msg).await?,
    };

    Ok(())
}

async fn make_poll(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    let option_menu = get_menu().await.unwrap_or_else(|_| None);
    let message = if let Some(menu) = option_menu
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
            last_poll_guard.insert(
                msg.chat.id.clone(), 
                RebekaPollData
                {
                    entrants_id: first_poll_message.poll().unwrap().id.clone(), 
                    seconds_id: second_poll_message.poll().unwrap().id.clone(), 
                    participants: Vec::new()
                });
        }

        second_poll_message
    }
    else 
    {
        bot.send_message(msg.chat.id, "El menu esta algo raro hoy, no he podido pasarlo a poll. Quejas a Roger!!").await?
    };
    Ok(message)
}

async fn who_calls(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    let text: String;
    unsafe
    {
        let last_polls = LAST_POLLS.lock().unwrap();
        if let Some(value) = last_polls.get(&msg.chat.id)
        {
            let random_caller = value.participants.choose(&mut rand::thread_rng());
            text = format!("Hoy llama {}!. Si ya has llamado recientemente, vuelve a usar /whocalls", random_caller.unwrap());
        }
        else 
        {
            text = "Nadie ha votado a las polls todavia, esperate a que almenos alguien haya votado!".to_string();
        }
    }
    Ok(bot.send_message(msg.chat.id, text).await?)
}

async fn rank_polls(bot: &Bot, msg: &Message, achivement_requested: &String) -> Result<Message, crate::RequestError>
{
    let text: String;
    unsafe
    {
        let last_polls = LAST_POLLS.lock().unwrap();
        if let Some(value) = last_polls.get(&msg.chat.id)
        {
            let random_caller = value.participants.choose(&mut rand::thread_rng());
            text = format!("Hoy llama {}!. Si ya has llamado recientemente, vuelve a usar /whocalls", random_caller.unwrap());
        }
        else 
        {
            text = "Nadie ha votado a las polls todavia, esperate a que almenos alguien haya votado!".to_string();
        }
    }
    Ok(bot.send_message(msg.chat.id, text).await?)
}

async fn wip(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    Ok(bot.send_message(msg.chat.id, "Work in progress").await?)
}

async fn answer_poll(_: Bot, poll_answer: PollAnswer) -> ResponseResult<()> 
{
    unsafe
    {
        let mut last_poll_guard = LAST_POLLS.lock().unwrap();
        last_poll_guard.values_mut().for_each(|x| 
        {
            if x.entrants_id == poll_answer.poll_id || x.seconds_id == poll_answer.poll_id
            {
                if poll_answer.option_ids.is_empty()
                {
                    let index = x.participants.iter().position(|x| *x == poll_answer.user.first_name).unwrap();
                    x.participants.remove(index);
                }
                else
                {
                    x.participants.push(poll_answer.user.first_name.clone());
                }
            }
        });
    }
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
    let mut xl_menu = all_entrants.get(0).unwrap().clone();
    xl_menu.push_str(" XL");
    all_entrants.insert(1, xl_menu);

    let mut all_seconds: Vec<String> = real_menu[second_plates..]
        .iter()
        .filter(|&x| !x.contains("*"))
        .map(|&x| x[..25].to_lowercase().to_string())
        .collect();
    all_seconds.iter_mut().for_each(|x| x.push_str("..."));
    Ok(Some(Menu{entrants: all_entrants, seconds: all_seconds}))
}
