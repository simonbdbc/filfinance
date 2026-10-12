// Gluestack est isolé ici ; les compositions métier ne connaissent pas ses créateurs.
import {
  Switch,
  ActivityIndicator,
  Pressable,
  Text,
  TextInput,
  View,
  Image,
  ScrollView,
  FlatList,
  VirtualizedList,
  SectionList,
} from "react-native";
import { createSwitch } from "@gluestack-ui/core/switch/creator";
import { createSelect } from "@gluestack-ui/core/select/creator";
import { createButton } from "@gluestack-ui/core/button/creator";
import { createInput } from "@gluestack-ui/core/input/creator";
import { createModal } from "@gluestack-ui/core/modal/creator";
import { createPressable } from "@gluestack-ui/core/pressable/creator";

export const UIButton = createButton({
  Root: Pressable,
  Text,
  Group: View,
  Spinner: ActivityIndicator,
  Icon: Image,
});
export const UIInput = createInput({
  Root: View,
  Input: TextInput,
  Icon: Image,
  Slot: Pressable,
});
export const UIDialog = createModal({
  Root: View,
  Content: View,
  CloseButton: Pressable,
  Header: View,
  Footer: View,
  Body: ScrollView,
  Backdrop: Pressable,
});
export const UIPressable = createPressable({ Root: Pressable });

export const UISelect = createSelect(
  { Root: View, Trigger: Pressable, Input: TextInput, Icon: Image },
  {
    Portal: UIDialog,
    Backdrop: UIDialog.Backdrop,
    Content: UIDialog.Content,
    DragIndicator: View,
    DragIndicatorWrapper: View,
    Item: Pressable,
    ItemText: Text,
    ScrollView,
    FlatList,
    VirtualizedList,
    SectionList,
    SectionHeaderText: Text,
  },
);

export const UISwitch = createSwitch({ Root: Switch });
