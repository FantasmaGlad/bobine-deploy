# Conservé pendant une mise à jour ($1 = 1), retiré à la désinstallation ($1 = 0).
if [ "$1" = 0 ]; then
    rm -f /usr/bin/bdp
fi
