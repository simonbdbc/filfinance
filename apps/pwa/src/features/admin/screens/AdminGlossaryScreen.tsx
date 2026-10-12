import { Switch } from "@shared/ui/Switch";
import { TextInput } from "@shared/ui/TextInput";
import { Text } from "@shared/ui/Text";
import type { ComponentProps, Dispatch, SetStateAction } from "react";
import { useState } from "react";
import {
  ActivityIndicator,
  Pressable,
  View,
  useWindowDimensions,
} from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type {
  TranslationGlossaryCreatePayload,
  TranslationGlossaryEntry,
  TranslationGlossaryPreviewRequest,
  TranslationGlossaryPreviewResponse,
  TranslationGlossarySystemRule,
} from "@shared/api/contracts";
import { formatLocalDateTimeWithRaw } from "@shared/lib/dateTime";
import { useThemeTokens } from "@shared/theme/theme";
import {
  AccessDeniedCard,
  Button,
  MetaText,
  SubtleText,
  UserProfileLoadingCard,
} from "@shared/ui";
import {
  CommandBar,
  DataGrid,
  ErrorState,
  InspectorPanel,
  KpiStrip,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
  TriPane,
} from "@shared/ui/Workspace";
import {
  type AdminGlossaryGridRow,
  useAdminGlossaryViewModel,
} from "../hooks/useAdminGlossaryViewModel";
import { useGlossaryEntries } from "../hooks/useGlossaryEntries";

type MatchMode = "exact" | "word_boundary" | "pattern";
type GlossaryScope = "full_pipeline" | "spoken_only";
type PreviewStage = "source_en" | "display_fr";

type MatchModeOption = {
  description: string;
  example: string;
  label: string;
  sourcePlaceholder: string;
  spokenPlaceholder: string;
  translatedPlaceholder: string;
  value: MatchMode;
};

type FormState = {
  caseSensitive: boolean;
  enabled: boolean;
  matchMode: MatchMode;
  notes: string;
  priority: string;
  scope: GlossaryScope;
  sourceTerm: string;
  spokenTerm: string;
  translatedTerm: string;
};

const emptyFormState: FormState = {
  caseSensitive: false,
  enabled: true,
  matchMode: "word_boundary",
  notes: "",
  priority: "0",
  scope: "full_pipeline",
  sourceTerm: "",
  spokenTerm: "",
  translatedTerm: "",
};

const MATCH_MODE_OPTIONS: readonly MatchModeOption[] = [
  {
    description:
      "Remplace seulement un terme complet, sans toucher aux sous-chaînes.",
    example: "ECB -> BCE",
    label: "Mot entier",
    sourcePlaceholder: "ECB",
    spokenPlaceholder: "Banque centrale européenne",
    translatedPlaceholder: "BCE",
    value: "word_boundary",
  },
  {
    description:
      "Remplace la séquence exacte, y compris quand elle est dans un texte plus long.",
    example: "Fed Chair -> président de la Fed",
    label: "Exact",
    sourcePlaceholder: "Fed Chair",
    spokenPlaceholder: "président de la Réserve fédérale",
    translatedPlaceholder: "président de la Fed",
    value: "exact",
  },
  {
    description:
      "Utilise <number> pour un groupe numérique ou <string> pour un élément sans ponctuation terminale.",
    example: "<number>h<number> -> <number:hour> <number:minute>",
    label: "Motif",
    sourcePlaceholder: "<number>h<number>",
    spokenPlaceholder: "<number:hour> <number:minute>",
    translatedPlaceholder: "<number>h<number>",
    value: "pattern",
  },
];

const SCOPE_OPTIONS: readonly {
  description: string;
  label: string;
  value: GlossaryScope;
}[] = [
  {
    description:
      "Protège la source, impose le FR affiché puis applique la forme vocale.",
    label: "Traduction + affichage + vocal",
    value: "full_pipeline",
  },
  {
    description:
      "Ignore la traduction et l’affichage ; remplace seulement avant Piper.",
    label: "Voix uniquement",
    value: "spoken_only",
  },
];

export function AdminGlossaryScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const glossary = useGlossaryEntries({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });
  const [form, setForm] = useState<FormState>(emptyFormState);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const [previewInput, setPreviewInput] = useState(
    "14:30. La BCE suit l’EUR/USD après la publication des NFP.",
  );
  const [previewStage, setPreviewStage] = useState<PreviewStage>("display_fr");
  const { width } = useWindowDimensions();
  const isWideLayout = width >= 1080;
  const viewModel = useAdminGlossaryViewModel({
    editingId,
    entries: glossary.entries,
    formEnabled: form.enabled,
    formMatchMode: form.matchMode,
    formPriority: form.priority,
  });

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  const resetForm = () => {
    setEditingId(null);
    setForm(emptyFormState);
    setFormError(null);
  };

  const save = async () => {
    const validationError = validateForm(form);
    if (validationError) {
      setFormError(validationError);
      return;
    }

    const payload = buildPayload(form);
    setFormError(null);
    const saved = editingId
      ? await glossary.updateEntry(editingId, payload)
      : await glossary.createEntry(payload);

    if (saved) {
      resetForm();
    }
  };

  const previewPipeline = async () => {
    const draftEntry = buildPreviewDraftEntry(form);
    if (draftEntry instanceof Error) {
      setFormError(draftEntry.message);
      return;
    }
    const payload: TranslationGlossaryPreviewRequest = {
      input_stage: previewStage,
      input_text: previewInput,
      ...(draftEntry ? { draft_entry: draftEntry } : {}),
    };
    setFormError(null);
    await glossary.previewPipeline(payload);
  };

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminGlossary}
      description="Pilotage du dictionnaire EN, des libellés FR et de la diction envoyée à Piper."
      commandBar={
        <CommandBar
          title="Dictionnaire et diction TTS"
          description="Gestion des règles métier, aperçu du texte et normalisation de la synthèse vocale."
          actions={[
            {
              disabled: glossary.isLoading,
              label: "Actualiser",
              onPress: glossary.reload,
            },
            {
              disabled:
                glossary.isLoading ||
                glossary.isSaving ||
                glossary.isReapplying,
              label: "Actualiser le glossaire",
              onPress: () => void glossary.reapplyTranslations(),
            },
          ]}
          meta={
            <>
              <StateIndicator tone="info">
                Actives : {viewModel.activeCount}
              </StateIndicator>
              <StateIndicator tone="muted">
                Inactives : {viewModel.inactiveCount}
              </StateIndicator>
              {glossary.isSaving ? (
                <StateIndicator tone="trace">enregistrement</StateIndicator>
              ) : null}
              {glossary.isReapplying ? (
                <StateIndicator tone="trace">actualisation</StateIndicator>
              ) : null}
            </>
          }
        />
      }
    >
      <KpiStrip items={viewModel.metrics} />

      <TriPane
        isWide={isWideLayout}
        railWidth={390}
        inspectorWidth={320}
        rail={
          <GlossaryFormPanel
            editingId={editingId}
            form={form}
            formError={formError}
            glossaryError={glossary.error}
            isLoading={glossary.isLoading}
            isSaving={glossary.isSaving || glossary.isReapplying}
            onRefresh={glossary.reload}
            onReset={resetForm}
            onSave={save}
            setForm={setForm}
          />
        }
        main={
          <GlossaryRegistryPanel
            editingId={editingId}
            isLoading={glossary.isLoading}
            isSaving={glossary.isSaving || glossary.isReapplying}
            onDisable={(entry) => void glossary.disableEntry(entry.id)}
            onEdit={(entry) => {
              setEditingId(entry.id);
              setForm(formFromEntry(entry));
              setFormError(null);
            }}
            rows={viewModel.rows}
          />
        }
        inspector={
          <>
            <InspectorPanel
              title="Détails"
              description="État courant du formulaire et du registre."
              fields={viewModel.inspectorFields}
            />
            <GlossaryPreviewPanel
              input={previewInput}
              isPreviewing={glossary.isPreviewing}
              onChangeInput={setPreviewInput}
              onPreview={previewPipeline}
              onSelectStage={setPreviewStage}
              preview={glossary.preview}
              stage={previewStage}
            />
            <GlossarySystemRulesPanel rules={glossary.systemRules} />
          </>
        }
      />
    </WorkspacePage>
  );
}

function GlossaryFormPanel({
  editingId,
  form,
  formError,
  glossaryError,
  isLoading,
  isSaving,
  onRefresh,
  onReset,
  onSave,
  setForm,
}: {
  editingId: number | null;
  form: FormState;
  formError: string | null;
  glossaryError: string | null;
  isLoading: boolean;
  isSaving: boolean;
  onRefresh: () => void;
  onReset: () => void;
  onSave: () => void;
  setForm: Dispatch<SetStateAction<FormState>>;
}) {
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();
  const modeMeta = matchModeMeta(form.matchMode);
  const isPatternMode = form.matchMode === "pattern";

  return (
    <SectionPanel
      title={editingId ? "Modifier une entrée" : "Nouvelle entrée"}
      description="Le champ vocal est appliqué uniquement à l’audio Piper."
      action={
        <Button disabled={isLoading} onPress={onRefresh}>
          Actualiser
        </Button>
      }
    >
      <View className="w-full max-w-full min-w-0 gap-3">
        <View className="flex-row flex-wrap gap-2">
          <StateIndicator tone={form.enabled ? "info" : "muted"}>
            {form.enabled ? "actif" : "inactif"}
          </StateIndicator>
          <StateIndicator tone="trace">
            {matchModeLabel(form.matchMode)}
          </StateIndicator>
          <StateIndicator tone={form.caseSensitive ? "trace" : "muted"}>
            {caseSensitivityLabel(form.caseSensitive)}
          </StateIndicator>
          <StateIndicator
            tone={form.scope === "spoken_only" ? "trace" : "info"}
          >
            {scopeLabel(form.scope)}
          </StateIndicator>
        </View>

        <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-3">
          <GlossaryInput
            containerClassName="min-w-[180px] flex-1"
            label="source_term"
            onChangeText={(sourceTerm) =>
              setForm((current) => ({ ...current, sourceTerm }))
            }
            placeholder={modeMeta.sourcePlaceholder}
            value={form.sourceTerm}
          />
          <GlossaryInput
            containerClassName="min-w-[180px] flex-1"
            label="translated_term"
            onChangeText={(translatedTerm) =>
              setForm((current) => ({ ...current, translatedTerm }))
            }
            placeholder={modeMeta.translatedPlaceholder}
            value={form.translatedTerm}
          />
        </View>

        <GlossaryInput
          label="spoken_term audio uniquement"
          onChangeText={(spokenTerm) =>
            setForm((current) => ({ ...current, spokenTerm }))
          }
          placeholder={modeMeta.spokenPlaceholder}
          value={form.spokenTerm}
        />
        <GlossaryInput
          keyboardType="numeric"
          label="priorité"
          onChangeText={(priority) =>
            setForm((current) => ({ ...current, priority }))
          }
          placeholder="0"
          value={form.priority}
        />

        <MatchModeSelector
          selectedMode={form.matchMode}
          onSelect={(matchMode) =>
            setForm((current) => ({
              ...current,
              matchMode,
            }))
          }
        />

        <ScopeSelector
          selectedScope={form.scope}
          onSelect={(scope) =>
            setForm((current) => ({
              ...current,
              scope,
            }))
          }
        />

        <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
          <ToggleRow
            label="Casse"
            value={form.caseSensitive}
            onValueChange={(caseSensitive) =>
              setForm((current) => ({ ...current, caseSensitive }))
            }
          />
          <ToggleRow
            label="Actif"
            value={form.enabled}
            onValueChange={(enabled) =>
              setForm((current) => ({ ...current, enabled }))
            }
          />
        </View>

        <View
          className="w-full max-w-full min-w-0 gap-1 border px-3 py-2"
          style={{
            backgroundColor: dashboardTheme.colors.surface,
            borderColor: dashboardTheme.colors.border,
            borderRadius: dashboardTheme.radii.control,
          }}
        >
          <MetaText>Mode {modeMeta.label}</MetaText>
          <SubtleText>{modeMeta.description}</SubtleText>
          <SubtleText>Exemple : {modeMeta.example}</SubtleText>
          {isPatternMode ? (
            <SubtleText>
              Utilisez exactement{" "}
              <Text style={{ color: tokens.colors.textPrimary }}>
                {"<number>"}
              </Text>{" "}
              ou{" "}
              <Text style={{ color: tokens.colors.textPrimary }}>
                {"<string>"}
              </Text>{" "}
              dans source_term, translated_term et spoken_term. Les captures
              doivent rester dans le même ordre ; spoken_term accepte aussi{" "}
              <Text style={{ color: tokens.colors.textPrimary }}>
                {"<number:hour>"}
              </Text>{" "}
              et{" "}
              <Text style={{ color: tokens.colors.textPrimary }}>
                {"<number:minute>"}
              </Text>{" "}
              pour la diction horaire ;
              <Text style={{ color: tokens.colors.textPrimary }}>
                {" <string>:"}
              </Text>{" "}
              permet par exemple de transformer{" "}
              <Text style={{ color: tokens.colors.textPrimary }}>
                {"NASDAQ:"}
              </Text>{" "}
              en{" "}
              <Text style={{ color: tokens.colors.textPrimary }}>
                {"NASDAQ :"}
              </Text>
              .
            </SubtleText>
          ) : null}
        </View>

        <GlossaryInput
          label="notes"
          multiline
          onChangeText={(notes) =>
            setForm((current) => ({ ...current, notes }))
          }
          placeholder="Contexte opérateur"
          value={form.notes}
        />

        {formError || glossaryError ? (
          <ErrorState message={formError ?? glossaryError ?? ""} />
        ) : null}

        <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
          <Button
            className="flex-1"
            disabled={isSaving}
            onPress={onSave}
            variant="primary"
          >
            {editingId ? "Enregistrer" : "Créer"}
          </Button>
          <Button className="flex-1" onPress={onReset}>
            Annuler
          </Button>
        </View>
        {isSaving ? (
          <ActivityIndicator size="small" color={tokens.colors.textPrimary} />
        ) : null}
      </View>
    </SectionPanel>
  );
}

function MatchModeSelector({
  onSelect,
  selectedMode,
}: {
  onSelect: (matchMode: MatchMode) => void;
  selectedMode: MatchMode;
}) {
  return (
    <View className="w-full max-w-full min-w-0 gap-1">
      <MetaText>Correspondance</MetaText>
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
        {MATCH_MODE_OPTIONS.map((option) => (
          <Button
            className="min-w-[112px] flex-1"
            key={option.value}
            onPress={() => onSelect(option.value)}
            variant={option.value === selectedMode ? "primary" : "secondary"}
          >
            {option.label}
          </Button>
        ))}
      </View>
    </View>
  );
}

function ScopeSelector({
  onSelect,
  selectedScope,
}: {
  onSelect: (scope: GlossaryScope) => void;
  selectedScope: GlossaryScope;
}) {
  return (
    <View className="w-full max-w-full min-w-0 gap-1">
      <MetaText>Portée</MetaText>
      <View className="w-full max-w-full min-w-0 gap-2">
        {SCOPE_OPTIONS.map((option) => (
          <Button
            key={option.value}
            onPress={() => onSelect(option.value)}
            textClassName="text-sm"
            variant={option.value === selectedScope ? "primary" : "secondary"}
          >
            {option.label}
          </Button>
        ))}
      </View>
      <SubtleText>
        {
          SCOPE_OPTIONS.find((option) => option.value === selectedScope)
            ?.description
        }
      </SubtleText>
    </View>
  );
}

function GlossaryRegistryPanel({
  editingId,
  isLoading,
  isSaving,
  onDisable,
  onEdit,
  rows,
}: {
  editingId: number | null;
  isLoading: boolean;
  isSaving: boolean;
  onDisable: (entry: AdminGlossaryGridRow) => void;
  onEdit: (entry: AdminGlossaryGridRow) => void;
  rows: AdminGlossaryGridRow[];
}) {
  return (
    <SectionPanel
      title="Registre du dictionnaire"
      description="Table compacte des règles métier, portées, priorités et variantes audio."
    >
      <DataGrid<AdminGlossaryGridRow>
        columns={[
          {
            key: "rule",
            label: "Règle",
            minWidth: 320,
            render: (entry) => <GlossaryRuleCell entry={entry} />,
          },
          {
            key: "meta",
            label: "Portée",
            minWidth: 190,
            render: (entry) => <GlossaryRulesCell entry={entry} />,
          },
          {
            key: "actions",
            label: "Actions",
            minWidth: 150,
            render: (entry) => (
              <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
                <CompactActionButton
                  disabled={isSaving}
                  onPress={() => onEdit(entry)}
                >
                  Modifier
                </CompactActionButton>
                {entry.enabled ? (
                  <CompactActionButton
                    disabled={isSaving}
                    onPress={() => onDisable(entry)}
                  >
                    Désactiver
                  </CompactActionButton>
                ) : (
                  <MetaText>Historique conservé.</MetaText>
                )}
              </View>
            ),
          },
        ]}
        density="compact"
        emptyLabel="Aucune entrée de glossaire."
        isLoading={isLoading}
        keyExtractor={(entry) => String(entry.id)}
        rows={rows}
        selectedRowKey={editingId ? String(editingId) : null}
      />
    </SectionPanel>
  );
}

function GlossaryRuleCell({ entry }: { entry: AdminGlossaryGridRow }) {
  const tokens = useThemeTokens();

  return (
    <View className="w-full max-w-full min-w-0 gap-1">
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap items-center gap-2">
        <StateIndicator tone={entry.statusTone}>
          {entry.enabled ? "actif" : "inactif"}
        </StateIndicator>
        <StateIndicator tone={entry.scope === "spoken_only" ? "trace" : "info"}>
          {entry.scopeLabel}
        </StateIndicator>
      </View>
      <Text
        className="max-w-full min-w-0 break-words text-sm font-extrabold"
        selectable
        style={{ color: tokens.colors.textPrimary }}
      >
        {entry.source_term}
        {" -> "}
        {entry.translated_term}
      </Text>
      <SubtleText>Voix : {entry.audioLabel}</SubtleText>
      {entry.notes ? <MetaText>{entry.notes}</MetaText> : null}
      <MetaText>{formatLocalDateTimeWithRaw(entry.updated_at)}</MetaText>
    </View>
  );
}

function GlossaryRulesCell({ entry }: { entry: AdminGlossaryGridRow }) {
  return (
    <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
      <StateIndicator tone="trace">
        {matchModeLabel(entry.match_mode)}
      </StateIndicator>
      <StateIndicator tone="muted">prio {entry.priority}</StateIndicator>
      <StateIndicator tone={entry.case_sensitive ? "trace" : "muted"}>
        {caseSensitivityLabel(entry.case_sensitive)}
      </StateIndicator>
    </View>
  );
}

function CompactActionButton({
  children,
  disabled,
  onPress,
}: {
  children: string;
  disabled?: boolean;
  onPress: () => void;
}) {
  const dashboardTheme = useThemeTokens();
  const tokens = useThemeTokens();

  return (
    <Pressable
      className="min-w-[58px] items-center border px-2 py-1.5"
      disabled={disabled}
      onPress={onPress}
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.border,
        borderRadius: dashboardTheme.radii.control,
        opacity: disabled ? 0.5 : 1,
      }}
    >
      <Text
        className="text-center text-sm font-bold"
        style={{ color: tokens.colors.textPrimary }}
      >
        {children}
      </Text>
    </Pressable>
  );
}

function GlossaryPreviewPanel({
  input,
  isPreviewing,
  onChangeInput,
  onPreview,
  onSelectStage,
  preview,
  stage,
}: {
  input: string;
  isPreviewing: boolean;
  onChangeInput: (value: string) => void;
  onPreview: () => void;
  onSelectStage: (stage: PreviewStage) => void;
  preview: TranslationGlossaryPreviewResponse | null;
  stage: PreviewStage;
}) {
  return (
    <SectionPanel
      title="Aperçu du traitement"
      description="Texte uniquement : aucun audio Piper n’est généré en v1."
      action={
        <Button
          disabled={isPreviewing || input.trim().length === 0}
          onPress={onPreview}
          textClassName="text-sm"
        >
          Tester
        </Button>
      }
    >
      <View className="w-full max-w-full min-w-0 gap-2">
        <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
          <Button
            className="min-w-[112px] flex-1"
            onPress={() => onSelectStage("display_fr")}
            textClassName="text-sm"
            variant={stage === "display_fr" ? "primary" : "secondary"}
          >
            FR affiché
          </Button>
          <Button
            className="min-w-[112px] flex-1"
            onPress={() => onSelectStage("source_en")}
            textClassName="text-sm"
            variant={stage === "source_en" ? "primary" : "secondary"}
          >
            EN source
          </Button>
        </View>
        <GlossaryInput
          label="texte test"
          multiline
          onChangeText={onChangeInput}
          placeholder="14:30. La BCE suit l’EUR/USD après la publication des NFP."
          value={input}
        />
        {isPreviewing ? <ActivityIndicator size="small" /> : null}
        {preview ? <GlossaryPreviewResult preview={preview} /> : null}
      </View>
    </SectionPanel>
  );
}

function GlossaryPreviewResult({
  preview,
}: {
  preview: TranslationGlossaryPreviewResponse;
}) {
  return (
    <View className="w-full max-w-full min-w-0 gap-2">
      <PreviewStep label="FR affiché" value={preview.display_text} />
      <PreviewStep label="FR vocal" value={preview.spoken_text} />
      <PreviewStep label="Texte Piper" value={preview.normalized_tts_text} />
      <StateIndicator tone={preview.glossary_hits > 0 ? "trace" : "muted"}>
        {preview.glossary_hits} hits
      </StateIndicator>
      {preview.warnings.map((warning) => (
        <SubtleText key={warning}>{warning}</SubtleText>
      ))}
    </View>
  );
}

function PreviewStep({ label, value }: { label: string; value: string }) {
  const dashboardTheme = useThemeTokens();
  const tokens = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-1 border px-2.5 py-2"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.borderSoft,
        borderRadius: dashboardTheme.radii.panel,
      }}
    >
      <MetaText>{label}</MetaText>
      <Text
        className="max-w-full min-w-0 break-words text-sm font-bold"
        selectable
        style={{ color: tokens.colors.textPrimary }}
      >
        {value}
      </Text>
    </View>
  );
}

function GlossarySystemRulesPanel({
  rules,
}: {
  rules: TranslationGlossarySystemRule[];
}) {
  return (
    <SectionPanel
      title="Règles système"
      description="Normalisations intégrées, visibles mais non modifiables."
    >
      <View className="w-full max-w-full min-w-0 gap-2">
        {rules.length === 0 ? (
          <SubtleText>Aucune règle système chargée.</SubtleText>
        ) : (
          rules.map((rule) => <SystemRuleItem key={rule.id} rule={rule} />)
        )}
      </View>
    </SectionPanel>
  );
}

function SystemRuleItem({ rule }: { rule: TranslationGlossarySystemRule }) {
  const tokens = useThemeTokens();

  return (
    <View className="w-full max-w-full min-w-0 gap-1 border-b pb-2">
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap items-center gap-2">
        <StateIndicator tone="muted">{rule.family}</StateIndicator>
        <Text
          className="max-w-full min-w-0 break-words text-sm font-extrabold"
          style={{ color: tokens.colors.textPrimary }}
        >
          {rule.label}
        </Text>
      </View>
      <MetaText>
        {rule.input_pattern}
        {" -> "}
        {rule.output_pattern}
      </MetaText>
      <SubtleText>{rule.description}</SubtleText>
      {rule.examples.slice(0, 2).map((example) => (
        <MetaText key={example}>{example}</MetaText>
      ))}
    </View>
  );
}

function GlossaryInput({
  containerClassName,
  label,
  ...props
}: {
  containerClassName?: string;
  label: string;
} & ComponentProps<typeof TextInput>) {
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();

  return (
    <View className={containerClassName ?? "w-full max-w-full min-w-0 gap-1"}>
      <MetaText>{label}</MetaText>
      <TextInput
        className="w-full max-w-full min-w-0 rounded-[12px] border px-3 py-2 text-sm"
        placeholderTextColor={tokens.colors.textPrimary}
        style={{
          backgroundColor: dashboardTheme.colors.surface,
          borderColor: dashboardTheme.colors.border,
          color: tokens.colors.textPrimary,
          borderRadius: dashboardTheme.radii.control,
          minHeight: props.multiline ? 78 : undefined,
          textAlignVertical: props.multiline ? "top" : "center",
        }}
        {...props}
      />
    </View>
  );
}

function ToggleRow({
  label,
  onValueChange,
  value,
}: {
  label: string;
  onValueChange: (value: boolean) => void;
  value: boolean;
}) {
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="min-w-[112px] flex-1 flex-row items-center justify-between border px-3 py-2"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.border,
        borderRadius: dashboardTheme.radii.control,
      }}
    >
      <MetaText>{label}</MetaText>
      <Switch
        accessibilityLabel={label}
        onValueChange={onValueChange}
        value={value}
      />
    </View>
  );
}

function buildPayload(form: FormState): TranslationGlossaryCreatePayload {
  const sourceTerm = form.sourceTerm.trim();
  const translatedTerm = form.translatedTerm.trim();

  return {
    case_sensitive: form.caseSensitive,
    enabled: form.enabled,
    match_mode: form.matchMode,
    notes: nullableTrim(form.notes),
    priority: Number.parseInt(form.priority.trim(), 10),
    scope: form.scope,
    source_language: "en",
    source_term: sourceTerm,
    spoken_term: nullableTrim(form.spokenTerm),
    target_language: "fr",
    translated_term: translatedTerm,
  };
}

function buildPreviewDraftEntry(form: FormState) {
  const hasDraftInput = Boolean(
    form.sourceTerm.trim() ||
    form.translatedTerm.trim() ||
    form.spokenTerm.trim() ||
    form.notes.trim(),
  );
  if (!hasDraftInput) {
    return null;
  }

  const validationError = validateForm(form);
  if (validationError) {
    return new Error(validationError);
  }

  return buildPayload(form);
}

function validateForm(form: FormState): string | null {
  const sourceTerm = form.sourceTerm.trim();
  const translatedTerm = form.translatedTerm.trim();
  const spokenTerm = form.spokenTerm.trim();
  const priority = form.priority.trim();

  if (!sourceTerm || !translatedTerm) {
    return "Renseignez au minimum source_term et translated_term.";
  }

  if (!priority || !/^-?\d+$/.test(priority)) {
    return "La priorité doit être un entier entre -10000 et 10000.";
  }

  const priorityValue = Number.parseInt(priority, 10);
  if (priorityValue < -10_000 || priorityValue > 10_000) {
    return "La priorité doit être comprise entre -10000 et 10000.";
  }

  if (form.matchMode !== "pattern") {
    return null;
  }

  const sourceCaptures = patternCaptures(sourceTerm);
  const translatedCaptures = patternCaptures(translatedTerm);
  const spokenCaptures = spokenTerm
    ? patternCaptures(spokenTerm, { allowNumberModifiers: true })
    : [];

  if (sourceCaptures.length === 0 || translatedCaptures.length === 0) {
    return "En mode Motif, source_term et translated_term doivent contenir au moins un <number> ou <string>.";
  }

  if (!sameCaptureSequence(sourceCaptures, translatedCaptures)) {
    return "En mode Motif, translated_term doit reprendre les captures de source_term dans le même ordre.";
  }

  if (spokenTerm && !sameCaptureSequence(sourceCaptures, spokenCaptures)) {
    return "En mode Motif, spoken_term doit reprendre les captures de source_term dans le même ordre.";
  }

  return null;
}

function patternCaptures(
  value: string,
  options: { allowNumberModifiers?: boolean } = {},
) {
  const capturePattern = options.allowNumberModifiers
    ? /<(number(?::(?:hour|minute))?|string)>/g
    : /<(number|string)>/g;
  return Array.from(value.matchAll(capturePattern)).map((match) =>
    match[1].startsWith("number") ? "number" : match[1],
  );
}

function sameCaptureSequence(left: string[], right: string[]) {
  return (
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function formFromEntry(entry: TranslationGlossaryEntry): FormState {
  return {
    caseSensitive: entry.case_sensitive,
    enabled: entry.enabled,
    matchMode: entry.match_mode,
    notes: entry.notes ?? "",
    priority: String(entry.priority),
    scope: entry.scope,
    sourceTerm: entry.source_term,
    spokenTerm: entry.spoken_term ?? "",
    translatedTerm: entry.translated_term,
  };
}

function nullableTrim(value: string) {
  const normalized = value.trim();
  return normalized ? normalized : null;
}

function matchModeLabel(matchMode: MatchMode) {
  return matchModeMeta(matchMode).label;
}

function scopeLabel(scope: GlossaryScope) {
  return SCOPE_OPTIONS.find((option) => option.value === scope)?.label ?? scope;
}

function matchModeMeta(matchMode: MatchMode) {
  return (
    MATCH_MODE_OPTIONS.find((option) => option.value === matchMode) ??
    MATCH_MODE_OPTIONS[0]
  );
}

function caseSensitivityLabel(caseSensitive: boolean) {
  return caseSensitive ? "Casse respectée" : "Casse ignorée";
}
