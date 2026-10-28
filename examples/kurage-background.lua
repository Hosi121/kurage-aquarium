local wezterm = require 'wezterm'

local kurage = {}

function kurage.apply(config, image_path)
  config.background = {
    {
      source = { Color = '#04090e' },
      width = '100%',
      height = '100%',
    },
    {
      source = {
        File = image_path or (wezterm.home_dir .. '/.config/kurage/aquarium.gif'),
      },
      opacity = 0.7,
      width = 'Cover',
      height = 'Cover',
      horizontal_align = 'Center',
      vertical_align = 'Middle',
      repeat_x = 'NoRepeat',
      repeat_y = 'NoRepeat',
      attachment = 'Fixed',
    },
  }
  return config
end

return kurage
