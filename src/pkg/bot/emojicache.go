package bot

import (
	"errors"

	dg "github.com/bwmarrin/discordgo"
)

// EmojiCache caches emoji values by GuildId for the doggo
type EmojiCache struct {
	cache  map[string]Emote
	config Config
}

// Emote represents the format for reacting with, or sending a message containing, this emote
type Emote struct {
	message string
	react   string
}

// NewEmojiCache builder
func NewEmojiCache(config Config) EmojiCache {
	return EmojiCache{
		cache:  make(map[string]Emote),
		config: config,
	}
}

func (ec *EmojiCache) get(s *dg.Session, m *dg.MessageCreate) (Emote, error) {
	emote, ok := ec.cache[m.GuildID]
	if !ok {
		emojis, err := s.GuildEmojis(m.GuildID)
		if err != nil {
			return Emote{}, err
		}
		emote, ok = ec.findEmoji(emojis)
		if !ok {
			return Emote{}, errors.New("Server does not have 'shrug_dog' emote")
		}
		ec.cache[m.GuildID] = emote
	}
	return emote, nil
}

func (ec *EmojiCache) findEmoji(emojis []*dg.Emoji) (Emote, bool) {
	for _, e := range emojis {
		if e.Name == ec.config.getEmoteName() {
			return Emote{e.MessageFormat(), e.APIName()}, true
		}
	}
	return Emote{}, false
}
