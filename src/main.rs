use std::{collections::HashMap, fs::File, io::Write, sync::{Arc, Mutex}};
use reqwest::{header::USER_AGENT, Client, Error};
use teloxide::{dispatching::UpdateFilterExt, prelude::*, types::{Chat, MessageKind, User}, update_listeners, utils::command::BotCommands, RequestError};
use html2text::from_read;
use once_cell::sync::Lazy;
use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};

struct Menu
{
    entrants: Vec<String>,
    seconds: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct Users
{
    chats_data: HashMap<ChatId, HashMap<UserId, PlayerScore>>,
}

impl Users
{
    pub fn new() -> Self
    {
        Users
        {
            chats_data: HashMap::new()
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PlayerScore
{
    user_name: String,
    polls_made: u16,
    calls_made: u16,
    xl_salads: u16,
    fastest_answering: u16,
    slowest_answering: u16,
    retracted_votes: u16,
}

impl PlayerScore
{
    pub fn new(name: String) -> Self
    {
        PlayerScore
        {
            user_name: name,
            polls_made: Default::default(),
            calls_made: Default::default(),
            xl_salads: Default::default(),
            fastest_answering: Default::default(),
            slowest_answering: Default::default(),
            retracted_votes: Default::default(),
        }
    }
}

struct RebekaPollAnswers
{
    entrants_selected: Vec<i32>,
    seconds_selected: Vec<i32>
}

impl RebekaPollAnswers
{
    pub fn new() -> Self{
        RebekaPollAnswers
        {
            entrants_selected: Vec::new(),
            seconds_selected: Vec::new(),
        }
    }
}

struct RebekaPollData
{
    chat_id: ChatId,
    entrants_id: String,
    entrants_options: Vec<String>,
    seconds_id: String,
    seconds_options: Vec<String>,
    participants: HashMap<User, RebekaPollAnswers>
}

static mut LAST_POLLS: Lazy<Mutex<HashMap::<ChatId, RebekaPollData>>> = Lazy::new(|| Mutex::new(HashMap::new()));
static mut CHAT_USERS: Lazy<Mutex<Users>> = Lazy::new(|| Mutex::new(Users::new()));
const JSON_FILE: &str = "users.json";

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
    #[command(description = "Muestra el pedido de forma simplificada.")]
    ShowOrder,
    #[command(description = "Cuando hayas hecho la llamada, recuerda de usar.")]
    CallMade,
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

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> 
{
    if let Some(user) = get_user_from(&msg)
    {
        ensure_user_exists(user, &msg.chat);
    }
    
    match cmd {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::MakePoll => make_poll(&bot, &msg).await?,
        Command::WhoCalls => who_calls(&bot, &msg).await?,
        Command::ShowOrder => show_order(&bot, &msg).await?,
        Command::CallMade => call_made(&bot, &msg).await?,
        Command::RankPolls => wip(&bot, &msg).await?,
        Command::RankCalls => wip(&bot, &msg).await?,
        Command::RankSaladsXL => wip(&bot, &msg).await?,
        Command::RankFastest => wip(&bot, &msg).await?,
        Command::RankSlowest => wip(&bot, &msg).await?,
        Command::RankRetracts => wip(&bot, &msg).await?,
    };

    Ok(())
}

fn get_user_from(msg: &Message) -> Option<&User>
{
    if let MessageKind::Common(common_message) = &msg.kind
    {
        common_message.from.as_ref()    
    }
    else
    {
        None
    }
}

fn ensure_user_exists(user: &User, chat: &Chat)
{
    unsafe
    {
        let mut chats = CHAT_USERS.lock().unwrap();
        {
            let mut file_result = File::open("users.json");
            if let Ok(json_file) = file_result.as_mut()
            {
                *chats = serde_json::from_reader(json_file).unwrap_or_default();
            }
        }
        let users = chats.chats_data.entry(chat.id.clone()).or_insert(HashMap::new());
        users.entry(user.id).or_insert(PlayerScore::new(user.first_name.clone()));
    }
    update_user_to_disk();
}

fn update_player_character<F>(user: &User, chat_id: &ChatId, mut f: F)
    where F : FnMut(&mut PlayerScore)
{
    unsafe
    {
        let mut chats = CHAT_USERS.lock().unwrap();
        let users = chats.chats_data.entry(chat_id.clone()).or_insert(HashMap::new());
        users.entry(user.id).and_modify(|x| f(x));
    }
    update_user_to_disk();
}

fn update_user_to_disk()
{
    let file_result = File::create(JSON_FILE);
    if let Ok(mut json_file) = file_result
    {
        unsafe
        {
            let chats = CHAT_USERS.lock().unwrap();
            let parsed_value = serde_json::to_string(&*chats);
            if let Ok(value) = parsed_value
            {
                json_file.write_all(value.as_bytes()).expect("Something when wrong when creating the json file of the users");
            }
        }
    }
}

async fn make_poll(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    let option_menu = get_menu().await.unwrap_or_else(|_| None);
    let message = if let Some(menu) = option_menu
    {
        let create_poll =  |question: String, options: Vec<String>| {
            let mut poll = bot.send_poll(msg.chat.id, question, options);
            poll.allows_multiple_answers = Some(true);
            poll.is_anonymous = Some(false);
            poll.disable_notification = Some(false);
            poll.send()
        };
        let first_poll_message = create_poll(String::from("Entrantes"), menu.entrants.clone()).await?;
        let second_poll_message = create_poll(String::from("Segundos"), menu.seconds.clone()).await?;

        unsafe
        {
            let mut last_poll_guard = LAST_POLLS.lock().unwrap();
            last_poll_guard.insert(
                msg.chat.id.clone(), 
                RebekaPollData
                {
                    chat_id: msg.chat.id.clone(),
                    entrants_id: first_poll_message.poll().unwrap().id.clone(),
                    entrants_options: menu.entrants.clone(),
                    seconds_id: second_poll_message.poll().unwrap().id.clone(), 
                    seconds_options: menu.seconds,
                    participants: HashMap::new(),
                });
        }

        if let Some(user) = get_user_from(&msg)
        {
            update_player_character(user, &msg.chat.id, |x| x.polls_made+=1);
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
        text = match last_polls.get(&msg.chat.id)
        {
            Some(value) if !value.participants.is_empty() => 
            {
                let users = value.participants.keys().collect::<Vec<&User>>();
                let random_caller = users.choose(&mut rand::thread_rng());
                format!("Hoy llama {}!. Si ya has llamado recientemente, vuelve a usar /whocalls", random_caller.unwrap().first_name)
            },
            _ => "Nadie ha votado a las polls todavia, esperate a que almenos alguien haya votado!".to_string()
        };
    }
    Ok(bot.send_message(msg.chat.id, text).await?)
}

async fn show_order(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    let text: String;
    unsafe
    {
        let last_polls = LAST_POLLS.lock().unwrap();
        text = if let Some(value) = last_polls.get(&msg.chat.id)
        {
            let mut entrants: HashMap<String, u32> = HashMap::new();
            let mut seconds: HashMap<String, u32> = HashMap::new();

            let register_dish = |selected: &Vec<i32>, options: &Vec<String>, dishes: &mut HashMap<String, u32>|
            {
                let last_index = options.len() as i32 - 1;
                let add_xl = selected.contains(&last_index);
                for index in selected
                {
                    if index == &last_index
                    {
                        continue;
                    }
                    let index = usize::try_from(index.clone()).expect("A negative index was passed as an option");
                    let mut dish = if add_xl {String::from("XL - ")} else {String::new()};
                    dish.push_str(options.get(index).unwrap().clone().as_str());
                    dishes.entry(dish).and_modify(|x| *x += 1).or_insert(1);
                }
            };

            for user in &value.participants
            {
                register_dish(&user.1.entrants_selected, &value.entrants_options, &mut entrants);
                register_dish(&user.1.seconds_selected, &value.seconds_options, &mut seconds);
            }

            let mut final_text: String = String::new();
            final_text.push_str("Entrantes\n");
            for value in &entrants
            {
                final_text.push_str(format!("{} - {}\n", value.1, value.0).as_str());
            }
            final_text.push_str("Seconds\n");
            for value in &seconds
            {
                final_text.push_str(format!("{} - {}\n", value.1, value.0).as_str());
            }
            final_text
        }
        else 
        {
            "No hay ningun plato pedido, esperate a que almenos alguien haya votado!".to_string()
        }
    }
    Ok(bot.send_message(msg.chat.id, text).await?)
}

async fn call_made(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    if let Some(user) = get_user_from(&msg)
    {
        update_player_character(user, &msg.chat.id, |x| x.calls_made+=1);
    }
    Ok(bot.send_message(msg.chat.id, String::from("Muchas gracias! Todos los que no habeis pedido, lo sentimos.")).await?)
}

async fn wip(bot: &Bot, msg: &Message) -> Result<Message, crate::RequestError>
{
    Ok(bot.send_message(msg.chat.id, "Aun no funciona, espera un poco porfavor!").await?)
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
                    update_player_character(&poll_answer.user, &x.chat_id, |x| x.retracted_votes+=1);
                }

                let entry = x.participants.entry(poll_answer.user.clone()).or_insert(RebekaPollAnswers::new());
                if x.entrants_id == poll_answer.poll_id
                {
                    entry.entrants_selected = poll_answer.option_ids.clone()
                }
                else 
                {
                    entry.seconds_selected = poll_answer.option_ids.clone()
                }
            }
        });
    }
    Ok(())
}

async fn get_menu() -> Result<Option<Menu>, Error>
{
    let menu_length = 50;

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

    let get_dishes_formated = |dishes: &[&str]|
    {
        let mut all_dishes: Vec<String> = dishes
            .iter()
            .filter(|&x| !x.contains("*"))
            .map(|&x| x[..usize::min(x.len(), menu_length)].to_lowercase().to_string())
            .collect();
        all_dishes.iter_mut().for_each(|x| x.push_str("..."));
        all_dishes.push("XL".to_string());
        all_dishes
    };
    Ok(Some(Menu{entrants: get_dishes_formated(&real_menu[..second_plates]), seconds: get_dishes_formated(&real_menu[second_plates..])}))
}
