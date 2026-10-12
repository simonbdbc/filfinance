import { Switch } from "@shared/ui/Switch";
import { TextInput } from "@shared/ui/TextInput";
import { Text } from "@shared/ui/Text";
import type { ComponentProps, Dispatch, SetStateAction } from "react";
import { useMemo, useState } from "react";
import { ActivityIndicator, View, useWindowDimensions } from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type {
  NewsBlacklistCreatePayload,
  NewsBlacklistEntry,
  NewsBlacklistHit,
  NewsBlacklistMatchMode,
  NewsBlacklistTargetScope,
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
import { useNewsBlacklist } from "../hooks/useNewsBlacklist";
import type { InspectorField, Metric } from "@shared/ui/workspaceTypes";

type FormState = {
  enabled: boolean;
  matchMode: NewsBlacklistMatchMode;
  notes: string;
  targetScope: NewsBlacklistTargetScope;
  term: string;
};

const emptyFormState: FormState = {
  enabled: true,
  matchMode: "partial",
  notes: "",
  targetScope: "title_content",
  term: "",
};

const MATCH_MODE_OPTIONS: readonly {
  description: string;
  example: string;
  label: string;
  value: NewsBlacklistMatchMode;
}[] = [
  {
    description: "Rejette l’actualité si le champ filtré contient le mot ou la phrase.",
    example: "sponsored content",
    label: "Partiel",
    value: "partial",
  },
  {
    description:
      "Rejette l’actualité uniquement si le mot ou la phrase apparaît avec des limites de mots.",
    example: "rumor",
    label: "Exact",
    value: "exact",
  },
];

const TARGET_SCOPE_OPTIONS: readonly {
  description: string;
  label: string;
  value: NewsBlacklistTargetScope;
}[] = [
  {
    description:
      "Teste uniquement le titre source en anglais et rejette l’actualité en cas de correspondance.",
    label: "Titre",
    value: "title",
  },
  {
    description:
      "Teste uniquement le contenu source en anglais ; le titre reste dans le fil et le contenu est retiré.",
    label: "Contenu",
    value: "content",
  },
  {
    description:
      "Teste le titre, puis le contenu source en anglais, et rejette l’actualité en cas de correspondance.",
    label: "Titre + contenu",
    value: "title_content",
  },
];

export function AdminBlacklistScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const blacklist = useNewsBlacklist({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });
  const [form, setForm] = useState<FormState>(emptyFormState);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [formError, setFormError] = useState<string | null>(null);
  const { width } = useWindowDimensions();
  const isWideLayout = width >= 1080;
  const viewModel = useBlacklistViewModel({
    editingId,
    entries: blacklist.entries,
    form,
    hits: blacklist.hits,
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
      ? await blacklist.updateEntry(editingId, payload)
      : await blacklist.createEntry(payload);

    if (saved) {
      resetForm();
    }
  };

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminBlacklist}
      description="Blocage des actualités sources en anglais avant leur enregistrement, leur traduction et leur conversion audio."
      commandBar={
        <CommandBar
          title="Liste de blocage à la collecte"
          description="Les règles actives filtrent le titre, le contenu ou les deux. L’actualisation s’applique à aujourd’hui et à hier."
          actions={[
            {
              disabled: blacklist.isLoading || blacklist.isReapplying,
              label: "Actualiser",
              onPress: blacklist.reload,
            },
            {
              disabled:
                blacklist.isLoading ||
                blacklist.isSaving ||
                blacklist.isReapplying,
              label: "Actualiser la liste de blocage",
              onPress: () => {
                void blacklist.reapplyBlacklist();
              },
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
              {blacklist.isSaving ? (
                <StateIndicator tone="trace">enregistrement</StateIndicator>
              ) : null}
              {blacklist.isReapplying ? (
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
        railWidth={370}
        inspectorWidth={320}
        rail={
          <BlacklistFormPanel
            editingId={editingId}
            form={form}
            formError={formError}
            isLoading={blacklist.isLoading || blacklist.isReapplying}
            isSaving={blacklist.isSaving || blacklist.isReapplying}
            onRefresh={blacklist.reload}
            onReset={resetForm}
            onSave={save}
            serverError={blacklist.error}
            setForm={setForm}
          />
        }
        main={
          <View className="w-full max-w-full min-w-0 gap-4">
            <BlacklistRegistryPanel
              editingId={editingId}
              isLoading={blacklist.isLoading}
              isSaving={blacklist.isSaving || blacklist.isReapplying}
              onDisable={(entry) => void blacklist.disableEntry(entry.id)}
              onEdit={(entry) => {
                setEditingId(entry.id);
                setForm(formFromEntry(entry));
                setFormError(null);
              }}
              rows={blacklist.entries}
            />
            <BlacklistHitsPanel
              hits={blacklist.hits}
              isLoading={blacklist.isLoading}
            />
          </View>
        }
        inspector={
          <InspectorPanel
            title="Détails de la liste de blocage"
            description="État courant des règles et rejets récents."
            fields={viewModel.inspectorFields}
          />
        }
      />
    </WorkspacePage>
  );
}

function BlacklistFormPanel({
  editingId,
  form,
  formError,
  isLoading,
  isSaving,
  onRefresh,
  onReset,
  onSave,
  serverError,
  setForm,
}: {
  editingId: number | null;
  form: FormState;
  formError: string | null;
  isLoading: boolean;
  isSaving: boolean;
  onRefresh: () => void;
  onReset: () => void;
  onSave: () => void;
  serverError: string | null;
  setForm: Dispatch<SetStateAction<FormState>>;
}) {
  const tokens = useThemeTokens();
  const mode = matchModeMeta(form.matchMode);

  return (
    <SectionPanel
      title={editingId ? "Modifier une règle" : "Nouvelle règle"}
      description="Mot ou phrase source en anglais, appliqué avant toute traduction."
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
          <StateIndicator tone="muted">
            {targetScopeLabel(form.targetScope)}
          </StateIndicator>
        </View>

        <BlacklistInput
          label="mot ou phrase source"
          onChangeText={(term) => setForm((current) => ({ ...current, term }))}
          placeholder={mode.example}
          value={form.term}
        />

        <MatchModeSelector
          onSelect={(matchMode) =>
            setForm((current) => ({ ...current, matchMode }))
          }
          selectedMode={form.matchMode}
        />

        <TargetScopeSelector
          onSelect={(targetScope) =>
            setForm((current) => ({ ...current, targetScope }))
          }
          selectedScope={form.targetScope}
        />

        <ToggleRow
          label="Actif"
          value={form.enabled}
          onValueChange={(enabled) =>
            setForm((current) => ({ ...current, enabled }))
          }
        />

        <ModeHelp mode={mode} />
        <TargetScopeHelp scope={targetScopeMeta(form.targetScope)} />

        <BlacklistInput
          label="notes"
          multiline
          onChangeText={(notes) =>
            setForm((current) => ({ ...current, notes }))
          }
          placeholder="Contexte opérateur"
          value={form.notes}
        />

        {formError || serverError ? (
          <ErrorState message={formError ?? serverError ?? ""} />
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
  onSelect: (matchMode: NewsBlacklistMatchMode) => void;
  selectedMode: NewsBlacklistMatchMode;
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

function TargetScopeSelector({
  onSelect,
  selectedScope,
}: {
  onSelect: (targetScope: NewsBlacklistTargetScope) => void;
  selectedScope: NewsBlacklistTargetScope;
}) {
  return (
    <View className="w-full max-w-full min-w-0 gap-1">
      <MetaText>Champ filtré</MetaText>
      <View className="w-full max-w-full min-w-0 flex-row flex-wrap gap-2">
        {TARGET_SCOPE_OPTIONS.map((option) => (
          <Button
            className="min-w-[128px] flex-1"
            key={option.value}
            onPress={() => onSelect(option.value)}
            variant={option.value === selectedScope ? "primary" : "secondary"}
          >
            {option.label}
          </Button>
        ))}
      </View>
    </View>
  );
}

function ModeHelp({ mode }: { mode: (typeof MATCH_MODE_OPTIONS)[number] }) {
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-1 border px-3 py-2"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.border,
        borderRadius: dashboardTheme.radii.control,
      }}
    >
      <MetaText>Mode {mode.label}</MetaText>
      <SubtleText>{mode.description}</SubtleText>
      <SubtleText>Exemple : {mode.example}</SubtleText>
    </View>
  );
}

function TargetScopeHelp({
  scope,
}: {
  scope: (typeof TARGET_SCOPE_OPTIONS)[number];
}) {
  const dashboardTheme = useThemeTokens();

  return (
    <View
      className="w-full max-w-full min-w-0 gap-1 border px-3 py-2"
      style={{
        backgroundColor: dashboardTheme.colors.surface,
        borderColor: dashboardTheme.colors.border,
        borderRadius: dashboardTheme.radii.control,
      }}
    >
      <MetaText>Champ {scope.label}</MetaText>
      <SubtleText>{scope.description}</SubtleText>
      {scope.value === "content" ? (
        <SubtleText>
          {"Si le filtre ne cible que le contenu, celui-ci est retiré de l’actualité en direct, mais son titre est conservé."}
        </SubtleText>
      ) : null}
    </View>
  );
}

function BlacklistRegistryPanel({
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
  onDisable: (entry: NewsBlacklistEntry) => void;
  onEdit: (entry: NewsBlacklistEntry) => void;
  rows: NewsBlacklistEntry[];
}) {
  return (
    <SectionPanel
      title="Liste de blocage"
      description="Règles appliquées au titre, au contenu ou aux deux avant stockage."
    >
      <DataGrid<NewsBlacklistEntry>
        columns={[
          {
            key: "term",
            label: "Règle",
            minWidth: 260,
            render: (entry) => <BlacklistTermCell entry={entry} />,
          },
          {
            key: "mode",
            label: "Mode",
            minWidth: 170,
            render: (entry) => (
              <View className="flex-row flex-wrap gap-2">
                <StateIndicator tone="trace">
                  {matchModeLabel(entry.match_mode)}
                </StateIndicator>
                <StateIndicator tone="muted">
                  {targetScopeLabel(entry.target_scope)}
                </StateIndicator>
                <StateIndicator tone={entry.enabled ? "info" : "muted"}>
                  {entry.enabled ? "actif" : "inactif"}
                </StateIndicator>
              </View>
            ),
          },
          {
            key: "notes",
            label: "Notes",
            minWidth: 220,
            render: (entry) => (
              <View className="gap-1">
                <SubtleText>{entry.notes ?? "n/a"}</SubtleText>
                <MetaText>
                  {formatLocalDateTimeWithRaw(entry.updated_at)}
                </MetaText>
              </View>
            ),
          },
          {
            key: "actions",
            label: "Actions",
            minWidth: 210,
            render: (entry) => (
              <View className="flex-row flex-wrap gap-2">
                <Button
                  className="min-w-[92px] flex-1"
                  disabled={isSaving}
                  onPress={() => onEdit(entry)}
                >
                  Modifier
                </Button>
                {entry.enabled ? (
                  <Button
                    className="min-w-[104px] flex-1"
                    disabled={isSaving}
                    onPress={() => onDisable(entry)}
                  >
                    Désactiver
                  </Button>
                ) : (
                  <MetaText>Historique conservé.</MetaText>
                )}
              </View>
            ),
          },
        ]}
        emptyLabel="Aucune règle de blocage."
        isLoading={isLoading}
        keyExtractor={(entry) => String(entry.id)}
        rows={rows}
        selectedRowKey={editingId ? String(editingId) : null}
      />
    </SectionPanel>
  );
}

function BlacklistTermCell({ entry }: { entry: NewsBlacklistEntry }) {
  const tokens = useThemeTokens();

  return (
    <View className="gap-1">
      <Text
        className="max-w-full min-w-0 break-words text-sm font-extrabold"
        style={{ color: tokens.colors.textPrimary }}
      >
        {entry.term}
      </Text>
      <SubtleText>
        Champ source en anglais : {targetScopeLabel(entry.target_scope).toLowerCase()}.
      </SubtleText>
    </View>
  );
}

function BlacklistHitsPanel({
  hits,
  isLoading,
}: {
  hits: NewsBlacklistHit[];
  isLoading: boolean;
}) {
  return (
    <SectionPanel
      title="Dernières actions de blocage"
      description="Audit des actualités rejetées ou des contenus retirés avant diffusion."
    >
      <DataGrid<NewsBlacklistHit>
        columns={[
          {
            key: "hit",
            label: "Rejet",
            minWidth: 260,
            render: (hit) => <BlacklistHitCell hit={hit} />,
          },
          {
            key: "source",
            label: "Source",
            minWidth: 170,
            render: (hit) => (
              <View className="gap-1">
                <StateIndicator tone="critical">{hit.source}</StateIndicator>
                <MetaText>{matchedFieldLabel(hit.matched_field)}</MetaText>
                <MetaText>{blacklistActionLabel(hit.action)}</MetaText>
              </View>
            ),
          },
          {
            key: "time",
            label: "Date",
            minWidth: 210,
            render: (hit) => (
              <View className="gap-1">
                <SubtleText>{hit.raw_payload_ref ?? "n/a"}</SubtleText>
                <MetaText>
                  {formatLocalDateTimeWithRaw(hit.created_at)}
                </MetaText>
              </View>
            ),
          },
        ]}
        emptyLabel="Aucun rejet récent lié à la liste de blocage."
        isLoading={isLoading}
        keyExtractor={(hit) => String(hit.id)}
        rows={hits}
      />
    </SectionPanel>
  );
}

function BlacklistHitCell({ hit }: { hit: NewsBlacklistHit }) {
  const tokens = useThemeTokens();

  return (
    <View className="gap-1">
      <View className="flex-row flex-wrap gap-2">
        <StateIndicator tone="critical">{hit.term}</StateIndicator>
        <StateIndicator tone="trace">
          {matchModeLabel(hit.match_mode)}
        </StateIndicator>
        <StateIndicator tone="muted">
          {targetScopeLabel(hit.target_scope)}
        </StateIndicator>
        <StateIndicator
          tone={hit.action === "strip_content" ? "info" : "critical"}
        >
          {blacklistActionLabel(hit.action)}
        </StateIndicator>
      </View>
      <Text
        className="max-w-full min-w-0 break-words text-sm font-bold leading-5"
        style={{ color: tokens.colors.textPrimary }}
      >
        {hit.source_title}
      </Text>
      <SubtleText>{hit.matched_text}</SubtleText>
    </View>
  );
}

function BlacklistInput({
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

function useBlacklistViewModel({
  editingId,
  entries,
  form,
  hits,
}: {
  editingId: number | null;
  entries: NewsBlacklistEntry[];
  form: FormState;
  hits: NewsBlacklistHit[];
}) {
  return useMemo(() => {
    const activeCount = entries.filter((entry) => entry.enabled).length;
    const inactiveCount = entries.length - activeCount;
    const exactCount = entries.filter(
      (entry) => entry.match_mode === "exact",
    ).length;

    const inspectorFields: InspectorField[] = [
      {
        label: "formulaire",
        tone: form.enabled ? "info" : "muted",
        value: form.enabled ? "actif" : "inactif",
      },
      { label: "mode", tone: "trace", value: matchModeLabel(form.matchMode) },
      {
        label: "champ",
        tone: "muted",
        value: targetScopeLabel(form.targetScope),
      },
      { label: "édition", value: editingId ? `#${editingId}` : "nouvelle" },
      {
        label: "rejets",
        tone: hits.length > 0 ? "critical" : "muted",
        value: hits.length,
      },
      {
        label: "exact",
        tone: exactCount > 0 ? "trace" : "muted",
        value: exactCount,
      },
    ];

    const metrics: Metric[] = [
      {
        description: "Règles appliquées pendant la collecte.",
        label: "Actives",
        tone: activeCount > 0 ? "info" : "muted",
        value: activeCount,
      },
      {
        description: "Règles actives et historiques.",
        label: "Total",
        tone: entries.length > 0 ? "trace" : "muted",
        value: entries.length,
      },
      {
        description: "Règles avec limites de mots.",
        label: "Exact",
        tone: exactCount > 0 ? "trace" : "muted",
        value: exactCount,
      },
      {
        description: "Dernières actions conservées en audit.",
        label: "Actions",
        tone: hits.length > 0 ? "critical" : "muted",
        value: hits.length,
      },
    ];

    return {
      activeCount,
      inactiveCount,
      inspectorFields,
      metrics,
    };
  }, [editingId, entries, form.enabled, form.matchMode, hits.length]);
}

function buildPayload(form: FormState): NewsBlacklistCreatePayload {
  return {
    enabled: form.enabled,
    match_mode: form.matchMode,
    notes: nullableTrim(form.notes),
    target_scope: form.targetScope,
    term: form.term.trim(),
  };
}

function validateForm(form: FormState): string | null {
  if (!form.term.trim()) {
    return "Renseignez un mot ou une phrase source.";
  }

  return null;
}

function formFromEntry(entry: NewsBlacklistEntry): FormState {
  return {
    enabled: entry.enabled,
    matchMode: entry.match_mode,
    notes: entry.notes ?? "",
    targetScope: entry.target_scope,
    term: entry.term,
  };
}

function nullableTrim(value: string) {
  const normalized = value.trim();
  return normalized ? normalized : null;
}

function matchModeLabel(matchMode: string) {
  return matchModeMeta(matchMode).label;
}

function matchModeMeta(matchMode: string) {
  return (
    MATCH_MODE_OPTIONS.find((option) => option.value === matchMode) ??
    MATCH_MODE_OPTIONS[0]
  );
}

function targetScopeLabel(targetScope: string) {
  return targetScopeMeta(targetScope).label;
}

function targetScopeMeta(targetScope: string) {
  return (
    TARGET_SCOPE_OPTIONS.find((option) => option.value === targetScope) ??
    TARGET_SCOPE_OPTIONS[2]
  );
}

function matchedFieldLabel(matchedField: string) {
  if (matchedField === "title") {
    return "Titre";
  }
  if (matchedField === "body") {
    return "Contenu";
  }
  return matchedField;
}

function blacklistActionLabel(action: string) {
  if (action === "strip_content") {
    return "contenu retiré";
  }
  if (action === "reject_news") {
    return "actualité rejetée";
  }
  return action;
}
