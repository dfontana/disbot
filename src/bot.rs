func (b *Bot) onMessageCreate(s *dg.Session, m *dg.MessageCreate) {
	// if m.Author.ID == s.State.User.ID {
	// 	// This message was from you
	// 	return
	// }
	if containsUser(m.Mentions, b.users) {
		emoji, err := b.emcache.get(s, m)
		if err != nil {
			fmt.Printf("Failed to pull emojis: %s, %s\n", m.GuildID, err)
			s.ChannelMessageSend(m.ChannelID, "You taketh my shrug, you taketh me :(")
			return
		}
		s.MessageReactionAdd(m.ChannelID, m.ID, emoji.react)
		s.ChannelMessageSend(m.ChannelID, emoji.message)
	}
}

// func containsUser(mentions []*dg.User, names map[string]bool) bool {
// 	for _, user := range mentions {
// 		if _, ok := names[user.Username]; ok {
// 			return true
// 		}
// 	}
// 	return false
// }
