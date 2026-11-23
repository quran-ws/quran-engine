// What the page view's styled pass draws, and the key of its base ink. The package does not
// export this file.
import 'types.dart';

/// One entry of the styled pass: a path of the draw list, the placement it is drawn under
/// there, and its colour.
typedef StyledDraw = ({int path, int placement, int color});

/// The entries of [draws] whose path is in [styled], in the draw list's order, each with its
/// own placement and its path's colour. A path the list holds twice, such as a line the layout
/// repeats over two rows, comes twice. A styled path the list does not hold does not come. A
/// hidden colour (alpha 0) does not come either: the base ink leaves out every styled path, so
/// the two passes draw each entry of the list once between them, and a hidden path not at all.
List<StyledDraw> styledDraws(List<QvpDraw> draws, List<QvpStyledPath> styled) {
  if (styled.isEmpty) return const [];
  final colorOf = <int, int>{for (final s in styled) s.path: s.color};
  return [
    for (final d in draws)
      if (colorOf[d.path] case final c? when (c & 0xff) != 0)
        (path: d.path, placement: d.placement, color: c),
  ];
}

/// A hash of which paths a draw list draws and under which placements. The base ink's
/// recording is keyed on it: with a letter layer, the list can change while the layout, the
/// band and the styled paths stay the same.
int drawListHash(List<QvpDraw> draws) {
  // kept under 2^30, so the hash is exact on the web, where an int is a double
  var h = draws.length;
  for (var i = 0; i < draws.length; i++) {
    h = (31 * h + draws[i].path) & 0x3fffffff;
    h = (31 * h + draws[i].placement) & 0x3fffffff;
  }
  return h;
}
