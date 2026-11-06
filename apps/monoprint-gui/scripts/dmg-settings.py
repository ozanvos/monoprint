"""Proof Sheet Finder layout, written without Finder/AppleScript automation."""
format = 'UDZO'
filesystem = 'HFS+'
files = [defines['app']]
symlinks = {'Applications': '/Applications'}
background = defines['background']
# Reserve room for Finder title, path and status bars around the 600×380 artwork.
window_rect = ((200, 200), (600, 460))
default_view = 'icon-view'
show_status_bar = True
show_tab_view = False
show_toolbar = False
show_pathbar = True
show_sidebar = False
include_icon_view_settings = True
include_list_view_settings = False
arrange_by = None
icon_size = 128
text_size = 12
label_pos = 'bottom'
icon_locations = {'Monoprint.app': (165, 176), 'Applications': (435, 176)}
