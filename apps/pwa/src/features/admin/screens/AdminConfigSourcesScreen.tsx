import { Switch } from "@shared/ui/Switch";
import { TextInput } from "@shared/ui/TextInput";
import { useState } from "react";
import { ActivityIndicator, View, useWindowDimensions } from "react-native";

import { appViewDisplayNameByView, useAppRuntime } from "@application";
import { useAuth } from "@features/auth";
import type { XRuntimeStatus } from "@shared/api/contracts";
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
  FilterRail,
  Grid,
  InspectorPanel,
  KpiStrip,
  WorkspacePage,
  SectionPanel,
  StateIndicator,
  TriPane,
} from "@shared/ui/Workspace";
import { useAdminSourceSettingsViewModel } from "../hooks/useAdminSourceSettingsViewModel";
import { useXRuntime } from "../hooks/useXRuntime";
import {
  normalizeXAccountInput,
  normalizeXKeywordInput,
} from "../services/dashboardFormatters";

type XRuntimeControl = {
  error: string | null;
  isLoading: boolean;
  isUpdating: boolean;
  reload: () => void;
  setAccountUsernames: (accountUsernames: string[]) => void;
  setEnabled: (enabled: boolean) => void;
  setKeywords: (keywords: string[]) => void;
  status: XRuntimeStatus | null;
};

type EditableRow = {
  id: string;
  label: string;
};

export function AdminConfigSourcesScreen() {
  const { authSession } = useAuth();
  const { canAccessAdmin, profile, role, server } = useAppRuntime();
  const xRuntime = useXRuntime({
    authSession,
    isEnabled: canAccessAdmin,
    wsUrl: server.wsUrl,
  });

  if (!profile.isUserProfileCurrent) {
    return <UserProfileLoadingCard error={profile.userProfileError} />;
  }

  if (!canAccessAdmin) {
    return <AccessDeniedCard role={role} />;
  }

  return <XSourceSettingsDashboard xRuntime={xRuntime} />;
}

function XSourceSettingsDashboard({ xRuntime }: { xRuntime: XRuntimeControl }) {
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();
  const { width } = useWindowDimensions();
  const isWideLayout = width >= 1080;
  const [accountInput, setAccountInput] = useState("");
  const [keywordInput, setKeywordInput] = useState("");
  const viewModel = useAdminSourceSettingsViewModel({
    error: xRuntime.error,
    isLoading: xRuntime.isLoading,
    isUpdating: xRuntime.isUpdating,
    status: xRuntime.status,
  });

  const addAccount = () => {
    const nextAccount = normalizeXAccountInput(accountInput);
    if (
      !nextAccount ||
      viewModel.accountUsernames.some(
        (item) => item.toLowerCase() === nextAccount.toLowerCase(),
      )
    ) {
      setAccountInput("");
      return;
    }

    xRuntime.setAccountUsernames([...viewModel.accountUsernames, nextAccount]);
    setAccountInput("");
  };

  const removeAccount = (account: string) => {
    xRuntime.setAccountUsernames(
      viewModel.accountUsernames.filter(
        (item) => item.toLowerCase() !== account.toLowerCase(),
      ),
    );
  };

  const addKeyword = () => {
    const nextKeyword = normalizeXKeywordInput(keywordInput);
    if (
      !nextKeyword ||
      viewModel.keywords.some(
        (item) => item.toLowerCase() === nextKeyword.toLowerCase(),
      )
    ) {
      setKeywordInput("");
      return;
    }

    xRuntime.setKeywords([...viewModel.keywords, nextKeyword]);
    setKeywordInput("");
  };

  const removeKeyword = (keyword: string) => {
    xRuntime.setKeywords(
      viewModel.keywords.filter(
        (item) => item.toLowerCase() !== keyword.toLowerCase(),
      ),
    );
  };

  return (
    <WorkspacePage
      title={appViewDisplayNameByView.adminConfigSources}
      description="Configuration enregistrée des sources de collecte."
      commandBar={
        <CommandBar
          title="Configuration de la source X"
          description="Activation, jeton d’accès, enregistrement et filtres de collecte."
          actions={[
            {
              disabled: xRuntime.isLoading,
              label: "Actualiser",
              onPress: xRuntime.reload,
            },
          ]}
          meta={
            <>
              {viewModel.statusBadges.map((badge) => (
                <StateIndicator key={badge.label} tone={badge.tone}>
                  {badge.label}
                </StateIndicator>
              ))}
              {xRuntime.isUpdating ? (
                <ActivityIndicator
                  size="small"
                  color={tokens.colors.textPrimary}
                />
              ) : null}
            </>
          }
        />
      }
    >
      <KpiStrip items={viewModel.metrics} />

      <TriPane
        isWide={isWideLayout}
        railWidth={300}
        inspectorWidth={330}
        rail={
          <FilterRail
            title="Sources"
            description="Accès aux paramètres des sources."
          >
            <View
              className="w-full max-w-full min-w-0 gap-3 border p-3"
              style={{
                backgroundColor: dashboardTheme.colors.surface,
                borderColor: dashboardTheme.colors.borderSoft,
                borderRadius: dashboardTheme.radii.panel,
              }}
            >
              <View className="flex-row flex-wrap gap-2">
                {viewModel.statusBadges.map((badge) => (
                  <StateIndicator key={badge.label} tone={badge.tone}>
                    {badge.label}
                  </StateIndicator>
                ))}
              </View>
              <MetaText>source: default</MetaText>
              <SubtleText>
                Source X synchronisée avec public.x_source_settings.
              </SubtleText>
              {viewModel.disabledReason ? (
                <SubtleText>
                  Coupure automatique : {viewModel.disabledReason}
                </SubtleText>
              ) : null}
              <View
                className="w-full max-w-full min-w-0 flex-row items-center justify-between gap-3 border px-3 py-2"
                style={{
                  backgroundColor: dashboardTheme.colors.surface,
                  borderColor: dashboardTheme.colors.border,
                  borderRadius: dashboardTheme.radii.control,
                }}
              >
                <MetaText>Service</MetaText>
                {xRuntime.isLoading ? (
                  <ActivityIndicator
                    size="small"
                    color={tokens.colors.textPrimary}
                  />
                ) : (
                  <Switch
                    accessibilityLabel="Activer la source X"
                    disabled={viewModel.isSwitchDisabled}
                    onValueChange={xRuntime.setEnabled}
                    value={viewModel.enabled}
                  />
                )}
              </View>
            </View>
          </FilterRail>
        }
        main={
          <Grid
            direction={isWideLayout ? "horizontal" : "vertical"}
            minItemWidth={330}
          >
            <EditableSourcePanel
              addLabel="Ajouter"
              disabled={viewModel.isMutationDisabled}
              emptyLabel="Aucun compte configuré."
              inputValue={accountInput}
              inputPlaceholder="@USTreasury"
              onAdd={addAccount}
              onInputChange={setAccountInput}
              onRemove={removeAccount}
              removeDisabled={(account) =>
                viewModel.isMutationDisabled ||
                viewModel.accountUsernames.length <= 1
              }
              rows={viewModel.accountRows}
              title="Comptes X"
              description="Comptes X suivis par le collecteur."
            />

            <EditableSourcePanel
              addLabel="Ajouter"
              disabled={viewModel.isMutationDisabled}
              emptyLabel="Aucun filtre par mot-clé."
              inputValue={keywordInput}
              inputPlaceholder="inflation"
              onAdd={addKeyword}
              onInputChange={setKeywordInput}
              onRemove={removeKeyword}
              rows={viewModel.keywordRows}
              title="Mots-clés"
              description="Filtres textuels appliqués au flux entrant."
            />
          </Grid>
        }
        inspector={
          <InspectorPanel
            title="Détails du service"
            description="Informations techniques fournies par l’API du service."
            fields={viewModel.inspectorFields}
          >
            {xRuntime.error ? <ErrorState message={xRuntime.error} /> : null}
          </InspectorPanel>
        }
      />
    </WorkspacePage>
  );
}

function EditableSourcePanel({
  addLabel,
  description,
  disabled,
  emptyLabel,
  inputPlaceholder,
  inputValue,
  onAdd,
  onInputChange,
  onRemove,
  removeDisabled,
  rows,
  title,
}: {
  addLabel: string;
  description: string;
  disabled: boolean;
  emptyLabel: string;
  inputPlaceholder: string;
  inputValue: string;
  onAdd: () => void;
  onInputChange: (value: string) => void;
  onRemove: (value: string) => void;
  removeDisabled?: (value: string) => boolean;
  rows: EditableRow[];
  title: string;
}) {
  const tokens = useThemeTokens();
  const dashboardTheme = useThemeTokens();

  return (
    <SectionPanel title={title} description={description}>
      <View className="w-full max-w-full min-w-0 gap-3">
        <View className="w-full max-w-full flex-row flex-wrap items-center gap-2">
          <TextInput
            autoCapitalize="none"
            autoCorrect={false}
            className="min-w-[190px] flex-1 border px-4 py-3 text-sm"
            editable={!disabled}
            onChangeText={onInputChange}
            onSubmitEditing={onAdd}
            placeholder={inputPlaceholder}
            placeholderTextColor={tokens.colors.textPrimary}
            style={{
              backgroundColor: dashboardTheme.colors.surface,
              borderColor: dashboardTheme.colors.border,
              borderRadius: dashboardTheme.radii.control,
              color: tokens.colors.textPrimary,
            }}
            value={inputValue}
          />
          <Button className="min-w-[96px]" disabled={disabled} onPress={onAdd}>
            {addLabel}
          </Button>
        </View>

        <DataGrid<EditableRow>
          columns={[
            {
              key: "label",
              label: "Valeur",
              minWidth: 220,
              render: (row) => row.label,
            },
            {
              key: "action",
              label: "Action",
              minWidth: 140,
              render: (row) => (
                <Button
                  className="min-w-[104px]"
                  disabled={disabled || removeDisabled?.(row.id)}
                  onPress={() => onRemove(row.id)}
                >
                  Retirer
                </Button>
              ),
            },
          ]}
          emptyLabel={emptyLabel}
          keyExtractor={(row) => row.id}
          rows={rows}
        />

        <SubtleText>Total : {rows.length}</SubtleText>
      </View>
    </SectionPanel>
  );
}
