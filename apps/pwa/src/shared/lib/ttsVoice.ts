type KnownSpeakerDetails = {
  name: string;
  gender: string;
};

const KNOWN_PIPER_SPEAKERS: Record<string, KnownSpeakerDetails> = {
  jessica: {
    name: "Jessica",
    gender: "femme",
  },
  pierre: {
    name: "Pierre",
    gender: "homme",
  },
};

const LOCALE_LABELS: Record<string, string> = {
  fr_BE: "Français (Belgique)",
  fr_CA: "Français (Canada)",
  fr_FR: "Français (France)",
  fr_CH: "Français (Suisse)",
};

const QUALITY_LABELS: Record<string, string> = {
  x_low: "qualité très basse",
  low: "qualité basse",
  medium: "qualité moyenne",
  high: "qualité élevée",
};

type ParsedTtsVoice = {
  raw: string;
  localeLabel: string;
  datasetLabel: string;
  qualityLabel: string;
  speakerLabel: string | null;
  genderLabel: string | null;
};

function capitalizeToken(value: string) {
  if (!value) {
    return value;
  }

  return value.charAt(0).toUpperCase() + value.slice(1);
}

function humanizeVoiceToken(value: string) {
  return value
    .split(/[_-]+/)
    .filter(Boolean)
    .map(capitalizeToken)
    .join(" ");
}

function parseTtsVoice(value: string): ParsedTtsVoice | null {
  const voice = value.trim();
  if (!voice) {
    return null;
  }

  const [locale, dataset, quality, ...speakerParts] = voice.split("-");
  if (!locale || !dataset || !quality) {
    return null;
  }

  const speaker = speakerParts.join("-");
  const speakerDetails = speaker ? KNOWN_PIPER_SPEAKERS[speaker] : null;

  return {
    raw: voice,
    localeLabel: LOCALE_LABELS[locale] || locale.replace("_", "-"),
    datasetLabel: dataset.toUpperCase(),
    qualityLabel: QUALITY_LABELS[quality] || `qualité ${quality}`,
    speakerLabel: speakerDetails?.name || (speaker ? humanizeVoiceToken(speaker) : null),
    genderLabel: speakerDetails?.gender || null,
  };
}

export function formatTtsVoiceShortLabel(voice: string) {
  const parsedVoice = parseTtsVoice(voice);
  if (!parsedVoice?.speakerLabel) {
    return voice;
  }

  return parsedVoice.genderLabel
    ? `${parsedVoice.speakerLabel} (${parsedVoice.genderLabel})`
    : parsedVoice.speakerLabel;
}

export function formatTtsVoiceOptionLabel(voice: string) {
  const parsedVoice = parseTtsVoice(voice);
  if (!parsedVoice) {
    return voice;
  }

  return [
    formatTtsVoiceShortLabel(voice),
    parsedVoice.localeLabel,
    parsedVoice.qualityLabel,
  ].join(" - ");
}

export function formatTtsVoiceDetailLabel(voice: string) {
  const parsedVoice = parseTtsVoice(voice);
  if (!parsedVoice) {
    return voice;
  }

  return [
    formatTtsVoiceShortLabel(voice),
    parsedVoice.localeLabel,
    parsedVoice.datasetLabel,
    parsedVoice.qualityLabel,
    parsedVoice.raw,
  ].join(" - ");
}
