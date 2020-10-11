package bot

import (
	"errors"

	dg "github.com/bwmarrin/discordgo"
)

// EmojiCache caches emoji values by GuildId for the doggo
type EmojiCache map[string]Emote

// Emote represents the format for reacting with, or sending a message containing, this emote
type Emote struct {
	message string
	react   string
}

// NewEmojiCache builder
func NewEmojiCache() EmojiCache {
	return make(EmojiCache)
}

func (ec EmojiCache) get(s *dg.Session, m *dg.MessageCreate) (Emote, error) {
	emote, ok := ec[m.GuildID]
	if !ok {
		emojis, err := s.GuildEmojis(m.GuildID)
		if err != nil {
			return Emote{}, err
		}
		emote, ok = findEmoji(emojis, "shrug_dog")
		if !ok {
			return Emote{}, errors.New("Server does not have 'shrug_dog' emote")
		}
		ec[m.GuildID] = emote
	}
	return emote, nil
}

func findEmoji(emojis []*dg.Emoji, name string) (Emote, bool) {
	for _, e := range emojis {
		if e.Name == name {
			return Emote{e.MessageFormat(), e.APIName()}, true
		}
	}
	return Emote{}, false
}
