use serenity::{
  builder::{CreateActionRow, CreateComponents},
  model::prelude::{ChannelId, ReactionType},
  utils::MessageBuilder,
};

use super::pollstate::PollState;
use humantime::format_duration;

pub fn build_poll_message(ps: &PollState) -> String {
  let mut bar_vec = ps
    .votes
    .iter()
    .map(|(idx, (opt, votes, _))| {
      format!(
        "{}: {:<opt_width$} | {:#<votes$}{:<bar_width$} | ({})",
        idx,
        opt,
        "",
        "",
        votes,
        votes = votes,
        opt_width = ps.longest_option,
        bar_width = ps.most_votes - votes
      )
    })
    .collect::<Vec<String>>();
  bar_vec.sort();

  let mut voter_vec = ps
    .votes
    .iter()
    .map(|(idx, (_, _, voters))| {
      format!(
        "{}: {}",
        idx,
        voters
          .iter()
          .map(|v| v.to_string())
          .collect::<Vec<String>>()
          .join(", ")
      )
    })
    .collect::<Vec<String>>();
  voter_vec.sort();

  MessageBuilder::new()
    .mention(&ps.ctx.emoji)
    .push_underline("Roommate Poll, Bobby, Roommate Poll!")
    .mention(&ps.ctx.emoji)
    .push_line("")
    .push_line("")
    .push_bold(&ps.topic)
    .push_italic(format!(" (exp in {})", format_duration(ps.duration)))
    .push_line("")
    .push_codeblock(
      format!(
        "{}\n\nVoters:\n{}",
        &bar_vec.join("\n"),
        voter_vec.join("\n")
      ),
      Some("m"),
    )
    .build()
}

pub async fn send_poll_message(ps: &PollState, itx: &ChannelId) -> serenity::Result<()> {
  itx
    .send_message(&ps.ctx.http, |builder| {
      let poll_msg = build_poll_message(ps);
      let mut component = CreateComponents::default();
      let mut action_row = CreateActionRow::default();

      action_row.create_select_menu(|select| {
        select
          .placeholder("Choose your Answers")
          .custom_id(ps.id)
          .min_values(1)
          .max_values(ps.votes.len() as u64)
          .options(|opts| {
            ps.votes.iter().for_each(|(k, v)| {
              opts.create_option(|opt| {
                opt
                  .label(v.0.to_owned())
                  .value(k.to_owned())
                  .emoji(ReactionType::Custom {
                    name: None,
                    animated: false,
                    id: ps.ctx.emoji.id,
                  })
              });
            });
            opts
          })
      });

      component.add_action_row(action_row);

      builder.content(poll_msg).set_components(component)
    })
    .await
    .map(|_| ())
}

pub fn build_exp_message(ps: &PollState) -> String {
  let winner = ps
    .votes
    .values()
    .max_by(|a, b| a.1.cmp(&b.1))
    .map(|v| v.0.to_string())
    .unwrap_or_else(|| "<Error Poll Had No Options?>".to_string());

  MessageBuilder::new()
    .mention(&ps.ctx.emoji)
    .push_underline("The Vote has Ended!")
    .mention(&ps.ctx.emoji)
    .push_line("")
    .push_line("")
    .push("The winner of \"")
    .push_bold(&ps.topic)
    .push("\" is: ")
    .push_bold(&winner)
    .push_line("")
    .push_italic("(Ties are resolved by the righteous power vested in me - deal with it)")
    .build()
}
