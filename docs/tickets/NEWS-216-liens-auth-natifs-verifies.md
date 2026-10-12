# NEWS-216 - Liens auth natifs verifies

## Statut

Bloque par les identifiants de signature et le provisioning des domaines mobiles.

## Risque confirme

Le scheme `filfinance://` n'est pas exclusif. Une autre application peut declarer
le meme scheme; iOS ne garantit alors pas quelle app recoit le lien. Better Auth
Expo ajoutait le cookie de session au callback d'activation, et le reset transportait
son token dans la query. Une interception locale permettait donc une prise de session
ou de compte.

La mitigation livree force les deux parcours vers la PWA HTTPS canonique et le
serveur reecrit tout callback fourni par le client. Elle ferme l'exposition des
secrets, mais ne restaure pas automatiquement la session dans l'app native.

## Cible

- iOS: Universal Links avec entitlement `applinks:` et fichier
  `apple-app-site-association` correspondant au Team ID + bundle ID;
- Android: App Links avec `android:autoVerify=true` et
  `/.well-known/assetlinks.json` contenant le package et les empreintes SHA-256;
- callback serveur sans cookie ni token Better Auth brut dans l'URL;
- code d'echange ephemere, mono-usage, tres court, lie au flow et a un verifier
  genere par l'app (PKCE ou preuve equivalente);
- echange du code contre la session seulement depuis l'app qui possede ce verifier;
- fallback navigateur PWA intact quand l'app n'est pas installee.

References officielles:

- [Apple - Custom URL schemes](https://developer.apple.com/documentation/Xcode/defining-a-custom-url-scheme-for-your-app)
- [Apple - Universal Links](https://developer.apple.com/documentation/xcode/allowing-apps-and-websites-to-link-to-your-content/)
- [Android - App Links](https://developer.android.com/training/app-links/about)

## Prerequis externes

- Apple Team ID et bundle identifier definitifs;
- package Android et empreintes des certificats dev, preview et production;
- controle des domaines PWA dev/prod et publication des deux fichiers
  d'association sans redirection;
- builds natifs installables signes pour les tests sur appareils reels.

## Acceptance

- activation et reset ouvrent la bonne app sur iOS et Android verifies;
- une app de test qui revendique `filfinance://` ne recoit aucun secret exploitable;
- cookie, token de reset et verifier ne figurent jamais dans une URL journalisee;
- le code d'echange expire, est mono-usage et echoue sans le verifier correct;
- l'activation restaure la session native et retire immediatement les parametres;
- les liens invalides/expirés restent lisibles dans l'app et dans le fallback web;
- tests E2E appareil couvrant app installee, absente et association invalide.
