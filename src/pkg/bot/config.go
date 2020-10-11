package bot

import (
	"fmt"
	"os"
	"strings"
)

// Config for Discord API
type Config struct {
	apiKey    string
	emoteName string
	users     []string
}

func (conf *Config) getKey() string {
	return conf.apiKey
}

func (conf *Config) getEmoteName() string {
	return conf.emoteName
}

func (conf *Config) getUsers() []string {
	return conf.users
}

// NewConfig builder
func NewConfig() (Config, error) {
	key, err := getEnv("API_KEY")
	if err != nil {
		return Config{}, err
	}
	emoteName, err := getEnv("EMOTE_NAME")
	if err != nil {
		return Config{}, err
	}
	usersToEmote, err := getEnv("EMOTE_USERS")
	if err != nil {
		return Config{}, err
	}
	return Config{key, emoteName, strings.Split(usersToEmote, ",")}, nil
}

func getEnv(key string) (string, error) {
	val := os.Getenv(key)
	if val == "" {
		return val, fmt.Errorf("Missing %s on this host", key)
	}
	return val, nil
}
