use std::sync::Arc;
use teloxide::{dispatching::UpdateFilterExt, prelude::*, update_listeners, utils::command::BotCommands};

mod rebeka_menu;
mod user_manager;
mod bot_commands;
mod translation_manager;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting command bot...");
    
    let bot = Bot::from_env();
    let def_handle = |_upd: Arc::<Update>| Box::pin(async {
    });
    
    Dispatcher::builder(
        bot.clone(),
        dptree::entry()
            .branch(Update::filter_message().filter_command::<Command>().endpoint(answer))
            .branch(Update::filter_poll_answer().endpoint(bot_commands::answer_poll)),
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
    #[command(description = "Check status")]
    AreYouAlive,
    #[command(description = "Crear la poll para entrantes y segundos.")]
    MakePoll,
    #[command(description = "Decide quien de los que hayan votado llama hoy.")]
    WhoCalls,
    #[command(description = "Bueno...")]
    WhoCallsTrueLegit,
    #[command(description = "Muestra el pedido de forma simplificada.")]
    ShowOrder,
    #[command(description = "Cuando hayas hecho la llamada, recuerda de usar.")]
    CallMade,
    #[command(description = "Si has traido tupper, llama este commando, seremos buenos.")]
    HoyTengoTupper,
    #[command(description = "Lista de ranking de polls creadas")]
    RankPolls,
    #[command(description = "Lista de ranking llamadas hechas")]
    RankCalls,
    #[command(description = "Lista de ranking de ensaladas XL pedidas")]
    RankXL,
    #[command(description = "Lista de ranking del que ha pedido mas rapido")]
    RankFastest,
    #[command(description = "Lista de ranking del que ha pedido mas lento")]
    RankSlowest,
    #[command(description = "Lista de ranking del que ha cambiado mas su voto")]
    RankRetracts,
    #[command(description = "Lista de ranking del que ha votado fuera de tiempo")]
    RankVeryLate,
    #[command(description = "Lista de la verguenza, quien ha traido mas tuppers")]
    RankTuppers,
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> 
{
    if let Some(user) = bot_commands::get_user_from(&msg)
    {
        user_manager::ensure_user_exists(user, &msg.chat);
    }
    
    match cmd 
    {
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::AreYouAlive => bot.send_message(msg.chat.id, translation_manager::get_i_am_alive()).await?,
        Command::MakePoll => bot_commands::make_poll(&bot, &msg).await?,
        Command::WhoCalls => bot_commands::who_calls(&bot, &msg).await?,
        Command::WhoCallsTrueLegit => bot.send_message(msg.chat.id, translation_manager::get_andrea_caller()).await?,
        Command::ShowOrder => bot_commands::show_order(&bot, &msg).await?,
        Command::CallMade => bot_commands::call_made(&bot, &msg).await?,
        Command::HoyTengoTupper => bot_commands::has_tupper(&bot, &msg).await?,
        Command::RankPolls => bot_commands::show_ranking_for(&bot, &msg, |x| x.polls_made, translation_manager::get_polls_created_title()).await?,
        Command::RankCalls => bot_commands::show_ranking_for(&bot, &msg, |x| x.calls_made, translation_manager::get_calls_made_title()).await?,
        Command::RankXL => bot_commands::show_ranking_for(&bot, &msg, |x| x.xl_dishes, translation_manager::get_xl_dishes_title()).await?,
        Command::RankFastest => bot_commands::show_ranking_for(&bot, &msg, |x| x.fastest_answering, translation_manager::get_fastest_voter_title()).await?,
        Command::RankSlowest => bot_commands::show_ranking_for(&bot, &msg, |x| x.slowest_answering, translation_manager::get_slower_voter_title()).await?,
        Command::RankRetracts => bot_commands::show_ranking_for(&bot, &msg, |x| x.retracted_votes, translation_manager::get_rectract_votes_title()).await?,
        Command::RankVeryLate => bot_commands::show_ranking_for(&bot, &msg, |x| x.out_of_time, translation_manager::get_vote_out_of_time_title()).await?,
        Command::RankTuppers => bot_commands::show_ranking_for(&bot, &msg, |x| x.tupper_count, translation_manager::get_tuppers_title()).await?,
    };

    Ok(())
}
