#!/bin/python
import gi
gi.require_version('Gtk', '3.0')
from gi.repository import Gtk
from gi.repository import Pango
import os
import mimetypes
import bliss
import csv
import sys
import threading

def analyze(url_lib, url_csv, self):
    if self.recursive:
        file_list = [os.path.join(dp, f) for dp, dn, fn in os.walk(os.path.expanduser(url_lib)) for f in fn]
    else:
        file_list = [os.path.join(url_lib, f) for f in os.listdir(url_lib) if os.path.isfile(os.path.join(url_lib, f))]

    audio_files = []
    
    for file_n in file_list:
        guess = mimetypes.guess_type(file_n)[0]
        if guess is not None and "audio" in guess:
            audio_files.append(file_n)

    if not audio_files:
        print("Please enter a valid directory containing audio files")
        self.goBtn.set_label("Go")
        return
       

    self.label_done.hide()
    self.progressBar.show()
    self.progressBar.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
    self.progressBar.set_show_text(True)

    with open(url_csv, "w") as csvfile:
        library_writer = csv.writer(csvfile, delimiter='|', quotechar="'", quoting=csv.QUOTE_MINIMAL)
        for i,file_n in enumerate(audio_files):
            if self.e.isSet():
                self.e.clear()
                break

            with bliss.bl_song(file_n) as song:
                # Test if the file has been decoded properly (ugly)
                if song["duration"] > 0:
                    self.progressBar.set_text(song["filename"])
                    library_writer.writerow((song["filename"], song["album"], song["force_vector"]["attack"], song["force_vector"]["tempo"], song["force_vector"]["amplitude"], song["force_vector"]["frequency"]))
                    csvfile.flush()
                else:
                    print("Couldn't decode file '%s', skipping..." % file_n)
            self.progressBar.set_fraction(i/(len(audio_files) - 1))

    self.progressBar.hide()
    self.label_done.set_label("Done!")
    self.label_done.show()
    self.goBtn.set_label("Go")
    print("Scan completed, data is availabe at '%s'" % url_csv)

class MyWindow(Gtk.Window):
    def __init__(self):
        Gtk.Window.__init__(self, title="Bliss data generator")
        self.url_csv = os.path.join(os.getcwd(), "output.csv")
        self.url_lib = ""
        self.th = None
        self.recursive = False
        self.e = threading.Event()
        self.progressBar = Gtk.ProgressBar()

        openDirBtn = Gtk.Button.new_with_label("Open...")
        openDirBtn.connect("clicked", self.onOpenDir)

        saveFileBtn = Gtk.Button.new_with_label("Save as CSV...")
        saveFileBtn.connect("clicked", self.onSaveFile)

        self.goBtn = Gtk.Button.new_with_label("Go")
        self.goBtn.connect("clicked", self.go)
        quitBtn = Gtk.Button.new_with_label("Quit")
        quitBtn.connect("clicked", self.quit)
       
        recursiveBtn = Gtk.CheckButton("Recursive scan")
        recursiveBtn.connect("toggled", self.recursiveCheck)

        self.label_open = Gtk.Label(self.url_lib)
        self.label_open.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
        self.label_save = Gtk.Label(self.url_csv)
        self.label_save.set_ellipsize(Pango.EllipsizeMode.MIDDLE)
        self.label_done = Gtk.Label()
       
        sizer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        sizer_open = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        sizer_save = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        sizer_set = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL)
        
        sizer.pack_start(sizer_open, True, True, 5)
        sizer.pack_start(sizer_save, True, True, 5)
        sizer.pack_start(self.progressBar, True, True, 5)
        sizer.pack_start(self.label_done, True, True, 5)
        sizer.pack_start(recursiveBtn, True, True, 5)
        sizer.pack_start(sizer_set, True, True, 5)

        sizer_open.pack_start(openDirBtn, False, True, 5)
        sizer_open.pack_start(self.label_open, True, True, 5)
        sizer_save.pack_start(saveFileBtn, False, True, 5)
        sizer_save.pack_start(self.label_save, True, True, 5)
        sizer_set.pack_start(quitBtn, True, True, 5)
        sizer_set.pack_start(self.goBtn, True, True, 5)

        self.add(sizer)

    def quit(self, evt):
        Gtk.main_quit()
          
    def go(self, button):
        if os.path.isabs(self.url_lib) and os.path.isabs(self.url_csv):
            if self.th is None or not self.th.isAlive():
                self.th = threading.Thread(target=analyze, args=(self.url_lib, self.url_csv, self))
                button.set_label("Cancel")
                self.th.start()
            else:
                button.set_label("Go")
                self.e.set()
        else:
            message = Gtk.MessageDialog(parent=self, flags=0, type=Gtk.MessageType.WARNING,
                buttons=Gtk.ButtonsType.NONE, message_format=None)
            message.set_markup("Please enter a valid directory containing audio files")
            message.show()

    def onOpenDir(self, evt):
        dialog = Gtk.FileChooserDialog("Please choose a folder to analyze", self,
            Gtk.FileChooserAction.SELECT_FOLDER,
            (Gtk.STOCK_CANCEL, Gtk.ResponseType.CANCEL, "Select", Gtk.ResponseType.OK))
        dialog.set_default_size(800, 400)
        response = dialog.run()

        if response == Gtk.ResponseType.OK:
            self.url_lib = dialog.get_filename()
            self.label_open.set_label(self.url_lib)

        dialog.destroy()

    def recursiveCheck(self, button):
        self.recursive = button.get_active()

    def onSaveFile(self, evt):
        dialog = Gtk.FileChooserDialog("Please choose an output CSV file", self,
            Gtk.FileChooserAction.SAVE,
            (Gtk.STOCK_CANCEL, Gtk.ResponseType.CANCEL, "Select", Gtk.ResponseType.OK))
        dialog.set_default_size(800, 400)
        response = dialog.run()

        if response == Gtk.ResponseType.OK:
            self.url_csv= dialog.get_filename()
            self.label_save.set_label(self.url_csv)

        dialog.destroy()

win = MyWindow()
win.connect("delete-event", Gtk.main_quit)
win.show_all()
win.progressBar.hide()
win.label_done.hide()

Gtk.main()
