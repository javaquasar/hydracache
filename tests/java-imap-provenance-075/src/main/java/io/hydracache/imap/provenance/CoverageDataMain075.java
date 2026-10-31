package io.hydracache.imap.provenance;

import java.io.*;
import java.nio.ByteBuffer;
import java.nio.charset.*;
import java.nio.file.*;
import java.util.*;
import javax.xml.XMLConstants;
import javax.xml.parsers.DocumentBuilderFactory;
import org.w3c.dom.Element;

/** Bounded JaCoCo source/session projection. No external DTD or entity resolution. */
public final class CoverageDataMain075 {
  private CoverageDataMain075() {}
  private static final String DTD="<!DOCTYPE report PUBLIC \"-//JACOCO//DTD Report 1.1//EN\" \"report.dtd\">";
  private static long number(Element element,String name) throws IOException {
    String value=element.getAttribute(name);
    if(!value.matches("[0-9]{1,19}"))throw new IOException("coverage integer missing/bounded");
    try{return Long.parseLong(value);}catch(NumberFormatException invalid){throw new IOException("coverage integer overflow",invalid);}
  }
  private static List<Element> children(Element element,String name){
    var result=new ArrayList<Element>();
    for(var node=element.getFirstChild();node!=null;node=node.getNextSibling())if(node instanceof Element child&&name.equals(child.getTagName())){
      if(result.size()>=20000)throw new IllegalArgumentException("coverage child work bound");result.add(child);
    }
    return result;
  }
  public static Map<String,Object> read(Path file) throws Exception {
    byte[] bytes;try(var input=Files.newInputStream(file)){bytes=input.readNBytes(16*1024*1024+1);}
    if(bytes.length==0||bytes.length>16*1024*1024)throw new IOException("coverage XML byte bound");
    String xml=StandardCharsets.UTF_8.newDecoder().onMalformedInput(CodingErrorAction.REPORT).decode(ByteBuffer.wrap(bytes)).toString();
    if(xml.contains("<!DOCTYPE")){
      int position=xml.indexOf(DTD);
      if(position<0||xml.indexOf("<!DOCTYPE",position+DTD.length())>=0||xml.indexOf("<!DOCTYPE")!=position)throw new IOException("foreign coverage DTD");
      xml=xml.substring(0,position)+xml.substring(position+DTD.length());
    }
    var factory=DocumentBuilderFactory.newInstance();factory.setNamespaceAware(true);factory.setXIncludeAware(false);factory.setExpandEntityReferences(false);
    factory.setFeature(XMLConstants.FEATURE_SECURE_PROCESSING,true);factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl",true);
    factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_DTD,"");factory.setAttribute(XMLConstants.ACCESS_EXTERNAL_SCHEMA,"");
    factory.setAttribute("http://www.oracle.com/xml/jaxp/properties/maxElementDepth","64");factory.setAttribute("http://www.oracle.com/xml/jaxp/properties/elementAttributeLimit","64");
    Element report=factory.newDocumentBuilder().parse(new ByteArrayInputStream(xml.getBytes(StandardCharsets.UTF_8))).getDocumentElement();
    if(!"report".equals(report.getTagName())||report.getNamespaceURI()!=null)throw new IOException("JaCoCo report root required");
    var sessions=new ArrayList<Object>();var ids=new HashSet<String>();
    for(var session:children(report,"sessioninfo")){
      String id=session.getAttribute("id");long start=number(session,"start"),dump=number(session,"dump");
      if(!id.matches("[A-Za-z0-9_-]{1,128}")||!ids.add(id)||dump<start||sessions.size()>=16)throw new IOException("coverage session identity/bounds");
      sessions.add(Map.of("id",id,"start",start,"dump",dump));
    }
    var sources=new TreeMap<String,Object>();long work=0;
    for(var pkg:children(report,"package")){
      String prefix=pkg.getAttribute("name");
      if(prefix.length()>512||!prefix.matches("(?:[A-Za-z_$][A-Za-z0-9_$]*(?:/[A-Za-z_$][A-Za-z0-9_$]*)*)?"))throw new IOException("foreign coverage package path");
      for(var source:children(pkg,"sourcefile")){
        String name=source.getAttribute("name");
        if(!name.matches("[A-Za-z_$][A-Za-z0-9_$-]*\\.java"))throw new IOException("foreign coverage source name");
        String path=prefix.isEmpty()?name:prefix+"/"+name;var lines=new TreeMap<String,Object>();
        for(var line:children(source,"line")){
          long nr=number(line,"nr"),covered=number(line,"ci"),missed=number(line,"mi");
          if(nr<1||nr>100000||covered>1000000000||missed>1000000000||covered+missed==0||++work>200000||lines.put(Long.toString(nr),covered)!=null)throw new IOException("duplicate/invalid coverage line");
        }
        if(sources.size()>=512||sources.put(path,lines)!=null)throw new IOException("duplicate/oversized coverage source set");
      }
    }
    if(sessions.isEmpty()||sources.isEmpty())throw new IOException("empty coverage sessions/sources");
    return Map.of("schema","imap075-java-coverage-v1","sessions",sessions,"sources",sources);
  }
  public static void main(String[] args)throws Exception{
    if(args.length!=1)throw new IllegalArgumentException("one JaCoCo XML input required");
    System.out.println(ProvenanceData075.json(read(Path.of(args[0]))));
  }
}
