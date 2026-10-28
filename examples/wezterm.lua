local wezterm = require 'wezterm'
local config = wezterm.config_builder()
local kurage = dofile(wezterm.home_dir .. '/.config/kurage/kurage-background.lua')

return kurage.apply(config)
