# Production OVH — FilFinance

Ce runbook cible une nouvelle production vierge sur le VPS OVH. Il ne migre
aucune donnee de l'ancien serveur et ne modifie ni les frontends, ni Bunny, ni
le Worker `filfinance-edge`.

## Topologie

```text
Internet :80/:443 -> Traefik -> filfinance-gateway -> Better Auth / Rust
                                            |
                                      Docker private network
```

`api.filfinance.fr` est l'unique host public du VPS. PostgreSQL, Redis, Better
Auth, Rust et le collector navigateur ne publient aucun port. Le gateway est
publie uniquement sur `127.0.0.1:8088` et joint Traefik via
`traefik_network`.

## Bootstrap serveur

L'image Ubuntu OVH fournit normalement un compte initial ayant `sudo` (souvent
`ubuntu`) ; une connexion SSH directe en `root` n'est ni necessaire ni
souhaitable. Dans les commandes ci-dessous, remplacer `ubuntu`, `IP_OVH` et
`22` par les informations communiquees par OVH.

### Utilisateur et deux cles distinctes

Sur le poste d'administration, creer exclusivement la cle GitLab CI -> OVH :

```bash
ssh-keygen -t ed25519 -a 100 -f ~/.ssh/id_ed25519_filfinance_ovh_ci -C 'gitlab-ci@filfinance-ovh'
```

Se connecter avec le compte initial et verifier ses privileges :

```bash
ssh -p 22 ubuntu@IP_OVH
sudo -v
```

Puis creer `gitlab-ci`, copier la cle publique sans jamais transmettre sa cle
privee, et verifier la seconde connexion par cle :

```bash
if ! id -u gitlab-ci >/dev/null 2>&1; then
  sudo useradd --create-home --shell /bin/bash --user-group gitlab-ci
fi
sudo install -d -m 700 -o gitlab-ci -g gitlab-ci /home/gitlab-ci/.ssh
exit

scp -P 22 ~/.ssh/id_ed25519_filfinance_ovh_ci.pub ubuntu@IP_OVH:/tmp/filfinance-ci.pub
ssh -p 22 ubuntu@IP_OVH 'sudo install -m 600 -o gitlab-ci -g gitlab-ci /tmp/filfinance-ci.pub /home/gitlab-ci/.ssh/authorized_keys && rm -f /tmp/filfinance-ci.pub && sudo passwd -l gitlab-ci'
ssh -i ~/.ssh/id_ed25519_filfinance_ovh_ci -p 22 gitlab-ci@IP_OVH 'id'
```

La cle inverse OVH -> GitLab est creee sur le VPS, apres la connexion
`gitlab-ci`, puis ajoutee dans GitLab comme *Deploy Key* lecture seule :

```bash
ssh -i ~/.ssh/id_ed25519_filfinance_ovh_ci -p 22 gitlab-ci@IP_OVH
ssh-keygen -t ed25519 -a 100 -f ~/.ssh/id_ed25519_filfinance_gitlab_ro -C 'filfinance-ovh-readonly'
cat ~/.ssh/id_ed25519_filfinance_gitlab_ro.pub
```

### Paquets, Docker et mises a jour

Toujours depuis le compte initial avec `sudo`, installer les prerequis. Le
depot Docker officiel est utilise, comme recommande par Docker pour une
installation de production :

```bash
sudo apt-get update
sudo apt-get install -y ca-certificates curl git openssl sqlite3 ufw unattended-upgrades
sudo install -m 0755 -d /etc/apt/keyrings
sudo curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc
sudo chmod a+r /etc/apt/keyrings/docker.asc
sudo tee /etc/apt/sources.list.d/docker.sources >/dev/null <<EOF
Types: deb
URIs: https://download.docker.com/linux/ubuntu
Suites: $(. /etc/os-release && echo "${UBUNTU_CODENAME:-$VERSION_CODENAME}")
Components: stable
Architectures: $(dpkg --print-architecture)
Signed-By: /etc/apt/keyrings/docker.asc
EOF
sudo apt-get update
sudo apt-get install -y docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin
sudo usermod -aG docker gitlab-ci
sudo systemctl enable --now docker
sudo dpkg-reconfigure -f noninteractive unattended-upgrades
sudo docker run --rm hello-world
docker compose version
```

Deconnecter ensuite `gitlab-ci` et se reconnecter afin que son groupe `docker`
soit pris en compte. Creer un swap de 4 Go (une seule fois) :

```bash
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
grep -qs '^/swapfile ' /etc/fstab || echo '/swapfile none swap sw 0 0' | sudo tee -a /etc/fstab
free -h
```

### Pare-feu et SSH

La premiere session initiale et la seconde session `gitlab-ci` doivent rester
ouvertes durant cette etape. Ouvrir d'abord le port SSH reel, puis HTTP et
HTTPS. Le gateway est lie a la boucle locale ; Docker ne publie donc pas la
base, Redis, Auth ou Rust sur Internet.

```bash
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp
sudo ufw --force enable
sudo ufw status verbose
```

Ne desactiver ni le mot de passe SSH ni l'acces root avant d'avoir effectivement
valide la seconde connexion par cle. Une fois celle-ci validee, ecrire une
configuration SSH dediee, la tester, puis recharger le service :

```bash
sudo tee /etc/ssh/sshd_config.d/90-filfinance-hardening.conf >/dev/null <<'EOF'
PasswordAuthentication no
KbdInteractiveAuthentication no
PermitRootLogin no
EOF
sudo sshd -t
sudo systemctl reload ssh
```

Creer le reseau une fois :

```bash
sudo docker network inspect traefik_network >/dev/null 2>&1 || sudo docker network create traefik_network
```

Creer enfin `/srv/filfinance`, attribue a `gitlab-ci`, puis cloner `main` avec
une cle GitLab de lecture distincte de la cle GitLab CI -> OVH.

```bash
sudo install -d -o gitlab-ci -g gitlab-ci -m 0750 /srv/filfinance
sudo -iu gitlab-ci git clone git@gitlab.com:simonbdbc/filfinance.git /srv/filfinance
```

## Configuration runtime

Creer `/srv/filfinance/.env.server.prod` a partir de `.env.server.example`,
avec un mode `600` et le proprietaire `gitlab-ci`. Renseigner des mots de passe
hexadecimaux distincts pour PostgreSQL, Better Auth, HLS et le token du
collector. Les URLs PostgreSQL doivent reprendre exactement ces mots de passe.
Le script suivant effectue cette creation sur le VPS sans afficher les secrets
et refuse d'ecraser un fichier deja existant :

```bash
cd /srv/filfinance
sudo -u gitlab-ci bash scripts/bootstrap_ovh_env.sh .env.server.prod .env.server.example
sudo stat -c '%a %U:%G %n' .env.server.prod
```

Conserver notamment :

```dotenv
FILFINANCE_ENV_FILE=.env.server.prod
FILFINANCE_AUTH_TRAEFIK_ENABLED=false
FILFINANCE_TRAEFIK_ENABLED=false
FILFINANCE_BACKEND_HOST_BIND=127.0.0.1
FILFINANCE_BACKEND_PORT=8088
NEWS_FINANCIAL_JUICE_BROWSER_HEADLESS=false
```

Le go-live est bloque tant que les placeholders de `SMTP_*`,
`NEWS_FINANCIAL_JUICE_*` et `OPENROUTER_API_KEY` ne sont pas remplaces. Edge,
Telegram et X restent desactives sans leurs identifiants respectifs.

Valider puis demarrer localement :

```bash
cd /srv/filfinance
bash scripts/migrate_server_env.sh .env.server.prod
bash scripts/check_deploy_env.sh .env.server.prod prod
bash scripts/check_better_auth_env.sh .env.server.prod prod
docker compose --env-file .env.server.prod -f compose.yaml -f compose.prod.yaml -f compose.ovh.yaml config >/dev/null
docker compose --env-file .env.server.prod -f compose.yaml -f compose.prod.yaml -f compose.ovh.yaml up -d --build
FILFINANCE_ENV_FILE=.env.server.prod FILFINANCE_COMPOSE_FILES=compose.yaml:compose.prod.yaml:compose.ovh.yaml bash scripts/verify_ovh_runtime.sh
curl -fsS http://127.0.0.1:8088/health
```

## Traefik

Copier `deploy/traefik/ovh` vers `/srv/traefik`, puis creer un fichier `.env`
avec `TRAEFIK_ACME_EMAIL` et `chmod 600 .env`. La configuration statique est
entierement portee par les arguments Compose de Traefik ; le fournisseur de
fichiers ne contient que la route dynamique de FilFinance. Creer `acme.json`
en tant que fichier regulier, egalement avec le mode `600` :

```bash
sudo install -d -m 0750 /srv/traefik
sudo cp -a /srv/filfinance/deploy/traefik/ovh/. /srv/traefik/
sudo touch /srv/traefik/acme.json
sudo chmod 600 /srv/traefik/acme.json /srv/traefik/.env
cd /srv/traefik
sudo docker compose --env-file .env -f compose.yaml config >/dev/null
sudo docker compose --env-file .env -f compose.yaml up -d
```

Traefik n'utilise pas le socket Docker. Sa configuration dynamique cible
explicitement `http://filfinance-gateway:8080`, donc le Nginx applicatif reste
le seul routeur entre Internet et Better Auth/Rust.

Le fichier est volontairement livre sous le nom
`dynamic/filfinance-api.yaml.disabled`, afin qu'aucune demande ACME ne parte
avant le cutover. Une fois le DNS `api.filfinance.fr` resolu vers OVH, l'activer
sans redemarrer Traefik :

```bash
cd /srv/traefik/dynamic
sudo mv filfinance-api.yaml.disabled filfinance-api.yaml
```

## GitLab CI

Associer le projet FilFinance au runner protege `home-runner-docker` de
`home-srv`. Les variables suivantes sont protegees, masquees lorsque GitLab le
permet, et limitees a l'environnement `production/backend` :

| Variable | Valeur |
| --- | --- |
| `HOST` | IP ou nom DNS du VPS OVH |
| `USER` | `gitlab-ci` |
| `SSH_PORT` | port SSH OVH configure |
| `SSH_PRIVATE_KEY` | cle privee CI -> OVH, encodee Base64 sur une ligne |
| `SSH_HOST_KEY_SHA256` | empreinte SHA256 de la cle hote SSH OVH |
| `FILFINANCE_PROD_DEPLOY_PATH` | `/srv/filfinance` |

Ne reutiliser aucune cle : la cle CI -> OVH donne l'acces SSH au runner,
tandis que la cle OVH -> GitLab est une deploy key GitLab en lecture seule.
La release reste manuelle et exige les gardes existantes
`FILFINANCE_PROD_RELEASE_REQUESTED`, `FILFINANCE_PROD_DEPLOY_ENABLED` et
`FILFINANCE_BETTER_AUTH_DEPLOY_ENABLED`.

## Bascule DNS et rollback

1. Noter les valeurs DNS actuelles de `api.filfinance.fr` et abaisser leur TTL.
2. Verifier que le backend et Traefik sont sains sur OVH, mais ne modifier le
   DNS qu'avec une decision operateur explicite. Avant cette decision, le
   controle strict des integrations doit passer sans afficher les secrets :

   ```bash
   cd /srv/filfinance
   bash scripts/check_ovh_go_live_env.sh .env.server.prod
   ```
3. Pointer `api.filfinance.fr` vers le VPS et attendre la resolution publique,
   puis renommer le fichier dynamique comme indique dans la section Traefik.
   Traefik obtient alors le certificat Let's Encrypt via HTTP-01.
4. Verifier :

   ```bash
   curl -fsS https://api.filfinance.fr/health
   curl -fsS https://api.filfinance.fr/api/auth/jwks
   ```

5. Lancer le smoke de production apres avoir fourni les comptes smoke Better
   Auth. Conserver l'ancien serveur au moins 72 heures. En cas d'incident
   d'infrastructure, restaurer le DNS precedent. Les donnees creees sur cette
   production vierge ne seront pas presentes sur l'ancien serveur.

## Sauvegarde locale provisoire

Installer le script et le timer :

```bash
sudo install -o root -g root -m 0755 /srv/filfinance/deploy/ovh/backup-filfinance.sh /usr/local/sbin/filfinance-backup
sudo install -o root -g root -m 0644 /srv/filfinance/deploy/ovh/systemd/filfinance-backup.service /etc/systemd/system/
sudo install -o root -g root -m 0644 /srv/filfinance/deploy/ovh/systemd/filfinance-backup.timer /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now filfinance-backup.timer
sudo systemctl start filfinance-backup.service
```

Le backup quotidien conserve sept jours dans `/srv/filfinance/backups` : dump
PostgreSQL, snapshot SQLite WAL coherent et verification d'integrite. Il ne
sauvegarde volontairement ni clips HLS, ni modeles, ni logs, ni profil
Chromium. Il ne protege donc pas contre la perte du VPS; activer Restic vers un
stockage hors site est un pre-requis a lever avant d'affirmer une production
resiliente. Installer `restic` maintenant, mais laisser tout transfert inactif
tant qu'un stockage n'est pas choisi ; le fichier
`deploy/ovh/restic.env.example` documente le futur fichier `/etc/filfinance/restic.env`
sans contenir de cible ni de credential actif.

Pour controler un backup :

```bash
sudo systemctl status filfinance-backup.timer
journalctl -u filfinance-backup.service -n 100 --no-pager
sudo -u gitlab-ci sqlite3 /srv/filfinance/backups/<timestamp>/news.db 'PRAGMA integrity_check;'
```
