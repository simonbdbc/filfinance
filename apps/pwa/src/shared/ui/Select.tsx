import { Platform } from "react-native";
import { UISelect } from "./primitives";
import { Icon } from "./Icon";
import { useThemeTokens } from "../theme/theme";
type SelectOption = { label: string; value: string; disabled?: boolean };
export function Select({
  value,
  disabled,
  options,
  onChange,
  label = "Sélectionner une option",
}: {
  value: string;
  disabled?: boolean;
  options: SelectOption[];
  onChange: (value: string) => void;
  label?: string;
}) {
  const t = useThemeTokens();
  return (
    <UISelect
      className="ff-select"
      selectedValue={value}
      isDisabled={disabled}
      onValueChange={onChange}
      initialLabel={options.find((o) => o.value === value)?.label}
      style={{ width: "100%", minWidth: 0 }}
    >
      <UISelect.Trigger
        role={Platform.OS === "web" ? "presentation" : "button"}
        aria-hidden={Platform.OS === "web" ? true : undefined}
        accessibilityLabel={label}
        style={{
          minHeight: 44,
          borderWidth: 1,
          borderColor: t.colors.border,
          borderRadius: 6,
          backgroundColor: t.colors.surface,
          flexDirection: "row",
          alignItems: "center",
          paddingHorizontal: 12,
        }}
      >
        <UISelect.Input
          placeholder={label}
          value={options.find((o) => o.value === value)?.label ?? ""}
          style={{
            flex: 1,
            minWidth: 0,
            fontFamily: t.fonts.body,
            fontSize: 16,
            color: t.colors.textPrimary,
            paddingVertical: 12,
          }}
        />
        <Icon name="chevron-down" size={18} />
      </UISelect.Trigger>
      <UISelect.Portal
        style={{ alignItems: "center", justifyContent: "center" }}
      >
        <UISelect.Backdrop
          style={{
            position: "absolute",
            inset: 0,
            backgroundColor: t.colors.overlay,
          }}
        />
        <UISelect.Content
          style={{
            maxHeight: "80%",
            padding: 16,
            backgroundColor: t.colors.surface,
            borderRadius: 8,
          }}
        >
          <UISelect.ScrollView>
            {options.map((o) => (
              <UISelect.Item
                key={o.value}
                label={o.label}
                value={o.value}
                isDisabled={o.disabled}
                style={{ minHeight: 44, padding: 12 }}
                textStyle={{
                  style: {
                    color: t.colors.textPrimary,
                    fontSize: 16,
                    fontFamily: t.fonts.body,
                  },
                }}
              />
            ))}
          </UISelect.ScrollView>
        </UISelect.Content>
      </UISelect.Portal>
    </UISelect>
  );
}
